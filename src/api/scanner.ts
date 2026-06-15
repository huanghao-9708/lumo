import { invoke } from '../utils/tauriInvoke';

export function sourceAddLocal(path: string, name: string): Promise<number> {
  return invoke('source_add_local', { path, name });
}

export function sourceAddWebdav(url: string, name: string, username?: string, password?: string): Promise<number> {
  return invoke('source_add_webdav', { url, name, username, password });
}

export function sourceList(): Promise<any[]> {
  return invoke('source_list');
}

export function sourceRemove(sourceId: number): Promise<void> {
  return invoke('source_remove', { sourceId });
}

export function sourceScan(sourceId: number): Promise<void> {
  return invoke('source_scan', { sourceId });
}
