import { invoke } from '../utils/tauriInvoke';

export interface SyncConfig {
  enabled: boolean;
  webdav_url?: string | null;
  username?: string | null;
  password?: string | null;
  remote_path?: string | null;
  last_sync_at?: string | null;
  last_sync_direction?: string | null;
}

export interface SyncResult {
  bytes_uploaded: number;
  timestamp: string;
}

export interface RemoteCheckResult {
  has_data: boolean;
  remote_size?: number | null;
  last_modified?: string | null;
}

export interface WebdavFile {
  path: string;
  is_dir: boolean;
  size: number;
  last_modified: string;
}

/** 获取同步配置 */
export function syncGetConfig(): Promise<SyncConfig> {
  return invoke('sync_get_config');
}

/** 保存同步配置 */
export function syncSaveConfig(config: SyncConfig): Promise<void> {
  return invoke('sync_save_config', { config });
}

/** 浏览 WebDAV 远程目录 */
export function syncBrowseWebdav(url: string, username: string | null, password: string | null, path: string): Promise<WebdavFile[]> {
  return invoke('sync_browse_webdav', { url, username, password, path });
}

/** 在 WebDAV 上新建文件夹 */
export function syncCreateFolder(url: string, username: string | null, password: string | null, path: string): Promise<void> {
  return invoke('sync_create_folder', { url, username, password, path });
}

/** 立即同步上传 */
export function syncUploadNow(): Promise<SyncResult> {
  return invoke('sync_upload_now');
}

/** 从云端恢复 */
export function syncRestoreNow(): Promise<string> {
  return invoke('sync_restore_now');
}

/** 检查云端是否有同步数据 */
export function syncCheckRemote(url: string, username: string | null, password: string | null, path: string): Promise<RemoteCheckResult> {
  return invoke('sync_check_remote', { url, username, password, path });
}
