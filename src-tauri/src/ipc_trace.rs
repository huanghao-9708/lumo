//! IPC 调用追踪工具（类似 Spring AOP 的环绕通知）。
//!
//! 用法：在每个 `#[tauri::command]` 函数体开头加一行：
//! ```ignore
//! #[tauri::command]
//! pub fn library_get_albums(...) -> Result<...> {
//!     let _trace = ipc_trace!("library_get_albums");
//!     // ... 业务逻辑
//! }
//! ```
//! RAII guard 在 drop 时自动打印耗时，无需在函数末尾再写任何代码。
//!
//! 日志与前端 `tauriInvoke.ts` 的 `[IPC]` 日志对应：
//! - 前端 `⏩ cmd` → 后端 `→ cmd ENTER`
//! - 前端 `✅ cmd Xms` → 后端 `← cmd EXIT (backend=Xms)`
//! 三者时间戳对齐，可拼出完整链路：前端发起 → 后端收到 → 后端返回 → 前端收到。

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// 全局 IPC 调用计数器，用于生成唯一请求序号，便于前后端日志关联。
static IPC_SEQ: AtomicU64 = AtomicU64::new(0);

/// 当前正在执行的 IPC 命令数量（含排队中）。
/// 与前端的 in_flight 计数对应，用于判断是否有命令在 Rust 侧排队。
static IN_FLIGHT: AtomicU64 = AtomicU64::new(0);

/// RAII guard：创建时记录入口，drop 时打印耗时。
pub struct IpcTraceGuard {
    cmd: &'static str,
    seq: u64,
    started: Instant,
    in_flight_at_enter: u64,
}

impl Drop for IpcTraceGuard {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        let in_flight_now = IN_FLIGHT.fetch_sub(1, Ordering::Relaxed);
        // 用毫秒和微秒组合，避免短任务显示 0ms 看不出差异
        let elapsed_str = if elapsed.as_millis() < 10 {
            format!("{}µs", elapsed.as_micros())
        } else {
            format!("{}ms", elapsed.as_millis())
        };
        // 慢调用（>100ms）用 WARN 级别，便于在日志里过滤
        if elapsed.as_millis() > 100 {
            tracing::warn!(
                "[IPC] ← {} EXIT seq={} backend={} (in_flight_at_enter={}, now={})",
                self.cmd, self.seq, elapsed_str, self.in_flight_at_enter, in_flight_now - 1
            );
        } else {
            tracing::info!(
                "[IPC] ← {} EXIT seq={} backend={}",
                self.cmd, self.seq, elapsed_str
            );
        }
    }
}

/// 创建一个 IPC 调用追踪 guard。
///
/// 在命令函数体第一行调用：`let _t = ipc_trace!("command_name");`
/// 函数返回时自动打印耗时，无需手动在末尾写日志。
#[macro_export]
macro_rules! ipc_trace {
    ($cmd:expr) => {{
        $crate::ipc_trace::__ipc_trace_inner($cmd)
    }};
}

/// 内部函数，由宏调用。不要直接调用，用 `ipc_trace!` 宏。
#[doc(hidden)]
pub fn __ipc_trace_inner(cmd: &'static str) -> IpcTraceGuard {
    let seq = IPC_SEQ.fetch_add(1, Ordering::Relaxed);
    let in_flight_at_enter = IN_FLIGHT.fetch_add(1, Ordering::Relaxed);
    tracing::info!(
        "[IPC] → {} ENTER seq={} (rust_in_flight={})",
        cmd, seq, in_flight_at_enter + 1
    );
    IpcTraceGuard {
        cmd,
        seq,
        started: Instant::now(),
        in_flight_at_enter,
    }
}
