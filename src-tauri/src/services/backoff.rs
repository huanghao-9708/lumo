//! 失败重试的退避策略。
//!
//! 存在的理由：网络类失败（429 限流、5xx 网关抖动、连接被切断）几乎总是成批出现，
//! 立即重试等于对着正在故障的服务端持续施压，而且会把错误事件按调用循环的频率灌给前端。
//! 这里只放纯计算部分——不 sleep、不发消息——好让退避序列能被单测钉住。

use std::time::{Duration, Instant};

/// 指数退避：第 N 次失败等待 `base × 2^(N-1)`，封顶 `max`。`failures == 0` 表示无需等待。
pub fn exp_backoff(failures: u32, base: Duration, max: Duration) -> Duration {
    if failures == 0 {
        return Duration::ZERO;
    }
    // 先夹住位移量再左移：failures 很大时 1 << n 会溢出成 0，那等于退回"无退避"，
    // 是这个模块要消灭的行为。
    let shift = (failures - 1).min(16);
    let factor = 1u32 << shift;
    base.saturating_mul(factor).min(max)
}

/// 连续失败计数器：每次失败给出本次应等待的时间，恢复后 `reset`。
#[derive(Debug, Clone)]
pub struct FailureBackoff {
    base: Duration,
    max: Duration,
    failures: u32,
}

impl FailureBackoff {
    pub fn new(base: Duration, max: Duration) -> Self {
        Self {
            base,
            max,
            failures: 0,
        }
    }

    /// 记一次失败并返回需要等待的时长。
    pub fn record_failure(&mut self) -> Duration {
        self.failures = self.failures.saturating_add(1);
        self.delay()
    }

    /// 当前失败次数对应的等待时长（不改变计数）。
    pub fn delay(&self) -> Duration {
        exp_backoff(self.failures, self.base, self.max)
    }

    /// 恢复正常（或用户已换操作）时清零，否则下一次偶发失败会继承历史惩罚。
    pub fn reset(&mut self) {
        self.failures = 0;
    }

    pub fn failures(&self) -> u32 {
        self.failures
    }
}

/// 自动切歌失败后的重试计划（CR-002）。
///
/// 观察者循环每 250ms 看一次「播完了没有」，没播起来就要决定下一跳做什么。三件事必须同时成立，
/// 「重试」才不会变成「跳过」：
/// - 重试目标锁定在**刚刚失败的那一首**，而不是从当前位置再往下数一首；
/// - 失败按指数退避排队，一旦播起来或用户改主意就整份作废；
/// - 错误事件按「曲目 + 错误类别」去重：同一首反复失败只报一次，换一首或换了故障类型必须再报。
///
/// 时间一律由调用方注入，本类型不 sleep、不发事件，因此退避窗口和去重都能被单测钉住。
#[derive(Debug, Clone)]
pub struct AdvanceRetry {
    backoff: FailureBackoff,
    /// 上次失败、退避结束后要原样重试的队列索引。
    pending: Option<usize>,
    next_attempt_at: Option<Instant>,
    last_error_key: Option<String>,
}

impl AdvanceRetry {
    pub fn new(base: Duration, max: Duration) -> Self {
        Self {
            backoff: FailureBackoff::new(base, max),
            pending: None,
            next_attempt_at: None,
            last_error_key: None,
        }
    }

    pub fn pending(&self) -> Option<usize> {
        self.pending
    }

    pub fn failures(&self) -> u32 {
        self.backoff.failures()
    }

    /// 退避窗口还没走完：这一跳应当什么都不做（不打网络、不发事件）。
    pub fn waiting(&self, now: Instant) -> bool {
        self.next_attempt_at.is_some_and(|at| now < at)
    }

    /// 记一次失败：把 `index` 锁成下次要重试的那一首，并排好下一次尝试的时间。
    /// 返回 `true` 表示这条「曲目 + 错误类别」是第一次出现，应当通知前端。
    pub fn record_failure(&mut self, index: usize, error_key: &str, now: Instant) -> bool {
        self.pending = Some(index);
        let delay = self.backoff.record_failure();
        self.next_attempt_at = Some(now + delay);
        let fresh = self.last_error_key.as_deref() != Some(error_key);
        if fresh {
            self.last_error_key = Some(error_key.to_string());
        }
        fresh
    }

    /// 播放恢复正常，或用户自己切歌／换队列／停止：旧的惩罚不能再拖累后面正常的切歌。
    pub fn clear(&mut self) {
        self.backoff.reset();
        self.pending = None;
        self.next_attempt_at = None;
        self.last_error_key = None;
    }
}

/// 可重试的 HTTP 状态：限流、超时与网关侧瞬时故障。
/// 401/403/404 不在此列——它们不会因为重试而变成成功。
pub fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

/// 解析 `Retry-After`（只支持 delta-seconds；HTTP-date 形式不解析，退回调用方的退避值）。
/// 结果封顶到 `max`：服务端可能给出分钟级等待，而调用方是在播放线程里等。
pub fn retry_after_delay(headers: &reqwest::header::HeaderMap, max: Duration) -> Option<Duration> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    let secs: u64 = raw.trim().parse().ok()?;
    Some(Duration::from_secs(secs).min(max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

    const SEC: Duration = Duration::from_secs(1);

    #[test]
    fn backoff_doubles_and_caps() {
        assert_eq!(exp_backoff(0, SEC, Duration::from_secs(30)), Duration::ZERO);
        assert_eq!(exp_backoff(1, SEC, Duration::from_secs(30)), SEC);
        assert_eq!(
            exp_backoff(2, SEC, Duration::from_secs(30)),
            Duration::from_secs(2)
        );
        assert_eq!(
            exp_backoff(5, SEC, Duration::from_secs(30)),
            Duration::from_secs(16)
        );
        assert_eq!(
            exp_backoff(9, SEC, Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn backoff_never_collapses_to_zero_on_extreme_counts() {
        // 溢出防护：u32::MAX 次失败必须仍等满上限，而不是移位回绕成 0
        assert_eq!(
            exp_backoff(u32::MAX, SEC, Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn failure_counter_accumulates_then_resets() {
        let mut b = FailureBackoff::new(SEC, Duration::from_secs(8));
        assert_eq!(b.record_failure(), SEC);
        assert_eq!(b.record_failure(), Duration::from_secs(2));
        assert_eq!(b.record_failure(), Duration::from_secs(4));
        assert_eq!(b.record_failure(), Duration::from_secs(8));
        assert_eq!(b.record_failure(), Duration::from_secs(8));
        assert_eq!(b.failures(), 5);
        b.reset();
        assert_eq!(b.failures(), 0);
        assert_eq!(b.record_failure(), SEC);
    }

    #[test]
    fn only_transient_statuses_are_retryable() {
        assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        ));
        assert!(!is_retryable_status(reqwest::StatusCode::OK));
        assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
    }

    #[test]
    fn retry_after_parsed_and_capped() {
        let mut h = HeaderMap::new();
        h.insert(RETRY_AFTER, HeaderValue::from_static("3"));
        assert_eq!(
            retry_after_delay(&h, Duration::from_secs(10)),
            Some(Duration::from_secs(3))
        );
        h.insert(RETRY_AFTER, HeaderValue::from_static("600"));
        assert_eq!(
            retry_after_delay(&h, Duration::from_secs(10)),
            Some(Duration::from_secs(10))
        );
        // HTTP-date 与非数字形式交给调用方的退避值
        h.insert(
            RETRY_AFTER,
            HeaderValue::from_static("Wed, 21 Oct 2026 07:28:00 GMT"),
        );
        assert_eq!(retry_after_delay(&h, Duration::from_secs(10)), None);
        assert_eq!(retry_after_delay(&HeaderMap::new(), SEC), None);
    }

    fn retry() -> AdvanceRetry {
        AdvanceRetry::new(SEC, Duration::from_secs(8))
    }

    /// CR-002 的核心：失败的那一首要被锁住并等满退避窗口，而不是立刻往后数一首。
    #[test]
    fn advance_retry_locks_the_failed_target() {
        let t0 = Instant::now();
        let mut r = retry();
        assert!(!r.waiting(t0), "没有失败时不该等待");
        assert_eq!(r.pending(), None);

        assert!(r.record_failure(3, "1:100:network", t0));
        assert_eq!(r.pending(), Some(3), "重试目标必须是刚失败的那一首");
        assert!(
            r.waiting(t0 + Duration::from_millis(500)),
            "1 秒退避窗口内不得再次尝试"
        );
        assert!(!r.waiting(t0 + Duration::from_millis(1001)));
    }

    #[test]
    fn advance_retry_waits_longer_each_time_and_caps() {
        let t0 = Instant::now();
        let mut r = retry();
        let mut at = t0;
        for (attempt, want_secs) in [1u64, 2, 4, 8, 8].into_iter().enumerate() {
            // 每次都换一个去重键，专注验证时间轴
            let key = format!("1:100:err{}", attempt);
            assert!(r.record_failure(1, &key, at));
            assert!(
                r.waiting(at + Duration::from_millis(want_secs * 1000 - 1)),
                "第 {} 次要等满 {} 秒",
                attempt + 1,
                want_secs
            );
            at += Duration::from_secs(want_secs);
            assert!(!r.waiting(at), "第 {} 次到点应放行", attempt + 1);
        }
        assert_eq!(r.failures(), 5);
        r.clear();
        assert_eq!(r.failures(), 0);
        assert_eq!(r.pending(), None);
        assert!(!r.waiting(t0));
    }

    /// 去重按「曲目 + 错误类别」：换一首歌、或同一首换了故障类型，都必须重新上报。
    #[test]
    fn advance_retry_dedups_per_track_and_error_class() {
        let t0 = Instant::now();
        let mut r = retry();
        assert!(r.record_failure(1, "1:100:network", t0));
        assert!(
            !r.record_failure(1, "1:100:network", t0),
            "同一首同一类错误只报第一次"
        );
        assert!(
            r.record_failure(2, "2:200:network", t0),
            "不同曲目的失败不能被合并掉"
        );
        assert!(
            r.record_failure(2, "2:200:not_found", t0),
            "同一首出现新的故障类型也要报"
        );
        r.clear();
        assert!(
            r.record_failure(2, "2:200:not_found", t0),
            "播起来之后同一类错误要能重新上报"
        );
    }
}
