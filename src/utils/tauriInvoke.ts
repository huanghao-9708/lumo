import { invoke as rawInvoke, type InvokeArgs } from '@tauri-apps/api/core';

/**
 * Tauri invoke 的 AOP 包装：类似 Spring AOP，对所有 IPC 调用做统一横切记录。
 *
 * 记录内容：命令名、参数摘要、耗时、返回值大小、当前在飞（in-flight）的 invoke 数量。
 * 其中 in-flight 计数是诊断"IPC 通道并发拥堵"的关键——如果某个 invoke 发出时
 * in-flight > 1，说明有其他 invoke 正在排队，可以解释"为什么后端快但前端慢"。
 *
 * 用法：把原来 `import { invoke } from '@tauri-apps/api/core'` 换成
 *      `import { invoke } from '../utils/tauriInvoke'`，调用代码完全不用改。
 *
 * 日志格式样例：
 *   [IPC] ⏩ library_get_albums (args: {limit:50,offset:50}) in_flight=2
 *   [IPC] ✅ library_get_albums 523ms ret=50items in_flight_was=2
 *
 * 问题定位后可以降低日志级别或移除此包装。
 */

/** 当前正在飞行（已发出、未返回）的 invoke 数量 */
let inFlightCount = 0;

/** Opt-in renderer/IPC timing probe. Enable in DevTools with:
 * localStorage.setItem('lumo_ipc_diagnostics', '1'); location.reload();
 */
const IPC_DIAGNOSTICS_ENABLED = (() => {
  try {
    return typeof window !== 'undefined'
      && window.localStorage.getItem('lumo_ipc_diagnostics') === '1';
  } catch {
    return false;
  }
})();

let ipcDiagnosticSequence = 0;

if (IPC_DIAGNOSTICS_ENABLED && typeof window !== 'undefined') {
  let previousBeat = performance.now();
  window.setInterval(() => {
    const now = performance.now();
    const gapMs = now - previousBeat;
    if (gapMs > 750) {
      console.warn(
        `[IPC-DIAG] renderer heartbeat gap=${Math.round(gapMs)}ms at=${new Date().toISOString()} visibility=${document.visibilityState}`,
      );
    }
    previousBeat = now;
  }, 250);

  if (typeof PerformanceObserver !== 'undefined') {
    try {
      const observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          console.warn(
            `[IPC-DIAG] long-task duration=${Math.round(entry.duration)}ms start=${Math.round(entry.startTime)}ms wall=${new Date().toISOString()}`,
          );
        }
      });
      observer.observe({ type: 'longtask', buffered: true });
    } catch {
      console.info('[IPC-DIAG] Long Task API unavailable');
    }
  }
}

/** 用于在慢调用时打出"当时还有谁在飞"，辅助定位并发冲突 */
interface InFlightRecord {
  cmd: string;
  startedAt: number;
}
const inFlightRecords: InFlightRecord[] = [];

const SENSITIVE_KEY = /pass(word)?|token|secret|credential|authorization|api[_-]?key/i;

/** Remove credentials before serialising IPC arguments/results for diagnostics. */
function redactForLog(value: unknown, key?: string): unknown {
  if (key && SENSITIVE_KEY.test(key)) return '[REDACTED]';
  if (Array.isArray(value)) return value.map(item => redactForLog(item));
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([entryKey, entryValue]) => [
        entryKey,
        redactForLog(entryValue, entryKey),
      ]),
    );
  }
  return value;
}

function summarizeArgs(args: unknown): string {
  if (args === undefined || args === null) return '∅';
  if (typeof args === 'object') {
    try {
      const json = JSON.stringify(redactForLog(args));
      // 参数太长就截断，避免日志爆炸
      return json.length > 200 ? json.slice(0, 200) + '…(' + json.length + 'B)' : json;
    } catch {
      return '[unserializable]';
    }
  }
  return String(args);
}

function summarizeResult(result: unknown): string {
  if (result === undefined || result === null) return 'null';
  if (typeof result === 'string') {
    return result.length > 300 ? `string(${result.length} chars)` : result;
  }
  if (Array.isArray(result)) return `${result.length}items`;
  if (typeof result === 'object') {
    try {
      const json = JSON.stringify(redactForLog(result));
      if (json.length > 300) return `${json.length}B`;
      return json;
    } catch {
      return '[obj]';
    }
  }
  return String(result);
}

function logStyle(elapsedMs: number): string {
  if (elapsedMs > 500) return 'color:#e22;font-weight:bold'; // 红：慢
  if (elapsedMs > 100) return 'color:#c80';                  // 黄：中
  return 'color:#0a0';                                        // 绿：快
}

export async function invoke<T = unknown>(cmd: string, args?: InvokeArgs): Promise<T> {
  const t0 = performance.now();
  const wallStart = Date.now();
  const diagnosticId = IPC_DIAGNOSTICS_ENABLED ? ++ipcDiagnosticSequence : 0;
  const myInFlight = inFlightCount;
  inFlightCount++;
  const myRecord: InFlightRecord = { cmd, startedAt: t0 };
  inFlightRecords.push(myRecord);

  if (IPC_DIAGNOSTICS_ENABLED) {
    console.info(
      `[IPC-DIAG] start id=${diagnosticId} cmd=${cmd} wall=${new Date(wallStart).toISOString()} mono=${Math.round(t0)}`,
    );
  }

  // 发出请求时记录：当前在飞数量（含自己）
  const othersInFlight = myInFlight; // 之前已经在飞的数量
  if (othersInFlight > 0) {
    // 有并发！列出其他在飞的命令，这对诊断"IPC 拥堵"至关重要
    const others = inFlightRecords
      .filter(r => r !== myRecord)
      .map(r => `${r.cmd}(${Math.round(t0 - r.startedAt)}ms ago)`);
    console.warn(
      `%c[IPC] ⏩ ${cmd} %c| ⚠️ 并发：当前还有 ${othersInFlight} 个 invoke 在飞：${others.join(', ')}`,
      'color:#08c',
      'color:#e22;font-weight:bold'
    );
  } else {
    console.log(`%c[IPC] ⏩ ${cmd} %cargs: ${summarizeArgs(args)}`, 'color:#08c', 'color:#888');
  }

  try {
    const result = await rawInvoke<T>(cmd, args);
    const elapsed = performance.now() - t0;

    if (IPC_DIAGNOSTICS_ENABLED) {
      console.info(
        `[IPC-DIAG] resolved id=${diagnosticId} cmd=${cmd} wall=${new Date().toISOString()} elapsed=${Math.round(elapsed)}ms`,
      );
    }

    console.log(
      `%c[IPC] ✅ ${cmd} %c${Math.round(elapsed)}ms %cret=${summarizeResult(result)}`,
      'color:#888',
      logStyle(elapsed),
      'color:#888'
    );
    return result;
  } catch (err) {
    const elapsed = performance.now() - t0;
    if (IPC_DIAGNOSTICS_ENABLED) {
      console.error(
        `[IPC-DIAG] rejected id=${diagnosticId} cmd=${cmd} wall=${new Date().toISOString()} elapsed=${Math.round(elapsed)}ms`,
      );
    }
    console.error(
      `%c[IPC] ❌ ${cmd} ${Math.round(elapsed)}ms ERROR:`,
      'color:#e22;font-weight:bold',
      err
    );
    throw err;
  } finally {
    inFlightCount--;
    const idx = inFlightRecords.indexOf(myRecord);
    if (idx >= 0) inFlightRecords.splice(idx, 1);
  }
}

// 导出原生类型供外部使用
export type { InvokeArgs } from '@tauri-apps/api/core';
