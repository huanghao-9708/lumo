import { defineStore } from 'pinia';
import { ref } from 'vue';
import {
  syncGetConfig,
  syncSaveConfig,
  syncBrowseWebdav,
  syncCreateFolder,
  syncUploadNow,
  syncRestoreNow,
  syncCheckRemote,
  type SyncConfig,
  type SyncResult,
  type RemoteCheckResult,
  type WebdavFile,
} from '../api/sync';

/**
 * 跨设备数据同步状态。
 *
 * 管理 WebDAV 同步配置的读写、文件夹浏览、上传/恢复操作。
 * 配置数据存于本地 SQLite 的 sync_config 表（密码用跨设备固定密钥加密）。
 */
export const useSyncStore = defineStore('sync', () => {
  // ===== 同步配置 =====
  const config = ref<SyncConfig>({
    enabled: false,
    webdav_url: null,
    username: null,
    password: null,
    remote_path: null,
    last_sync_at: null,
    last_sync_direction: null,
  });
  const isLoaded = ref(false);

  // ===== 操作状态 =====
  const isSyncing = ref(false);
  const isRestoring = ref(false);
  const isBrowsing = ref(false);
  const isChecking = ref(false);

  // ===== 文件夹浏览器结果 =====
  const browseEntries = ref<WebdavFile[]>([]);
  const browseError = ref('');

  // ===== 错误/提示 =====
  const lastError = ref('');
  const lastResult = ref('');

  /** 从后端加载同步配置 */
  async function fetchConfig() {
    try {
      const c = await syncGetConfig();
      config.value = c;
      isLoaded.value = true;
    } catch (e: any) {
      lastError.value = `加载同步配置失败: ${e}`;
    }
  }

  /** 保存同步配置到后端 */
  async function saveConfig() {
    try {
      await syncSaveConfig(config.value);
      lastError.value = '';
      lastResult.value = '同步配置已保存';
    } catch (e: any) {
      lastError.value = `保存失败: ${e}`;
    }
  }

  /** 浏览远程目录（按路径列出子目录） */
  async function browseWebdav(path: string) {
    if (!config.value.webdav_url) return;
    isBrowsing.value = true;
    browseError.value = '';
    try {
      browseEntries.value = await syncBrowseWebdav(
        config.value.webdav_url,
        config.value.username || null,
        config.value.password || null,
        path,
      );
    } catch (e: any) {
      browseError.value = `浏览失败: ${e}`;
      browseEntries.value = [];
    } finally {
      isBrowsing.value = false;
    }
  }

  /** 在远程路径下新建文件夹 */
  async function createFolder(name: string) {
    if (!config.value.webdav_url) return;
    try {
      const targetPath = config.value.remote_path
        ? `${config.value.remote_path.replace(/\/$/, '')}/${name}`
        : `/${name}`;
      await syncCreateFolder(
        config.value.webdav_url,
        config.value.username || null,
        config.value.password || null,
        targetPath,
      );
      return targetPath;
    } catch (e: any) {
      browseError.value = `新建文件夹失败: ${e}`;
      return null;
    }
  }

  /** 立即同步上传 */
  async function uploadNow(): Promise<SyncResult | null> {
    isSyncing.value = true;
    lastError.value = '';
    lastResult.value = '';
    try {
      const result = await syncUploadNow();
      config.value.last_sync_at = result.timestamp;
      config.value.last_sync_direction = 'upload';
      lastResult.value = `同步完成（${(result.bytes_uploaded / 1024 / 1024).toFixed(1)} MB）`;
      return result;
    } catch (e: any) {
      lastError.value = `同步失败: ${e}`;
      return null;
    } finally {
      isSyncing.value = false;
    }
  }

  /** 从云端恢复 */
  async function restoreNow(): Promise<boolean> {
    isRestoring.value = true;
    lastError.value = '';
    lastResult.value = '';
    try {
      const msg = await syncRestoreNow();
      lastResult.value = msg;
      // 恢复后重新加载配置（新 DB 可能有不同的配置）
      await fetchConfig();
      return true;
    } catch (e: any) {
      lastError.value = `恢复失败: ${e}`;
      return false;
    } finally {
      isRestoring.value = false;
    }
  }

  /** 检查云端是否有同步数据 */
  async function checkRemote(): Promise<RemoteCheckResult | null> {
    if (!config.value.webdav_url) return null;
    isChecking.value = true;
    try {
      return await syncCheckRemote(
        config.value.webdav_url,
        config.value.username || null,
        config.value.password || null,
        config.value.remote_path || '/',
      );
    } catch (_e) {
      return null;
    } finally {
      isChecking.value = false;
    }
  }

  return {
    config,
    isLoaded,
    isSyncing,
    isRestoring,
    isBrowsing,
    isChecking,
    browseEntries,
    browseError,
    lastError,
    lastResult,
    fetchConfig,
    saveConfig,
    browseWebdav,
    createFolder,
    uploadNow,
    restoreNow,
    checkRemote,
  };
});
