import { invoke } from '../utils/tauriInvoke';
import type { SourceDTO } from './types';

export function sourceAddLocal(path: string, name: string): Promise<number> {
  return invoke('source_add_local', { path, name });
}

export function sourceAddWebdav(url: string, name: string, username?: string, password?: string): Promise<number> {
  return invoke('source_add_webdav', { url, name, username, password });
}

/**
 * `scan-complete` 事件载荷，对应 Rust `services::scanner::ScanCompletePayload`（CR-006）。
 * 后端保证：扫描线程启动后一定会发出一条本事件，成功与失败都有终态。
 */
export interface ScanCompleteEvent {
  source_id: number;
  success: boolean;
  /** 稳定的机器可读分类（如 credential_unresolved / db_unavailable），不随文案变化 */
  error_code?: string | null;
  /** 面向用户的中文原因，与写入 sources.last_error 的内容一致 */
  message?: string | null;
  /** false = 终态没能写进数据库（数据库当时不可用），前端必须直接展示 message */
  persisted: boolean;
}

export interface WebdavProbeResult {
  ok: boolean;
  latencyMs: number;
  serverHeader?: string;
  statusCode?: number;
  error?: string;
}

export function scannerTestWebdav(url: string, username?: string, password?: string): Promise<WebdavProbeResult> {
  return invoke('scanner_test_webdav', { url, username, password });
}

export function sourceList(): Promise<SourceDTO[]> {
  return invoke('source_list');
}

export function sourceRemove(sourceId: number): Promise<void> {
  return invoke('source_remove', { sourceId });
}

export function sourceScan(sourceId: number): Promise<void> {
  return invoke('source_scan', { sourceId });
}
