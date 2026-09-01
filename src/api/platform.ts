import { invoke } from '../utils/tauriInvoke';

/**
 * MA1：移动平台桥 API。
 *
 * 桌面构建下这些命令返回安全默认值（UI 由 isAndroid 门控，不会触达）。
 */

/** "granted" | "denied" */
export function checkAudioPermission(): Promise<string> {
  return invoke('platform_check_audio_permission');
}

/** 发起运行时申请，结果经 `lumo-permission-result` 事件异步回传（payload: boolean）。 */
export function requestAudioPermission(): Promise<void> {
  return invoke('platform_request_audio_permission');
}

export function openAppSettings(): Promise<void> {
  return invoke('platform_open_app_settings');
}

export function storageSuggestions(): Promise<string[]> {
  return invoke('platform_storage_suggestions');
}

export function getAppVersion(): Promise<string> {
  return invoke('app_get_version');
}
