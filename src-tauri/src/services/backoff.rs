//! 失败重试的退避策略。
//!
//! 存在的理由：网络类失败（429 限流、5xx 网关抖动、连接被切断）几乎总是成批出现，
//! 立即重试等于对着正在故障的服务端持续施压，而且会把错误事件按调用循环的频率灌给前端。
//! 这里只放纯计算部分——不 sleep、不发消息——好让退避序列能被单测钉住。

use std::time::Duration;

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
}
