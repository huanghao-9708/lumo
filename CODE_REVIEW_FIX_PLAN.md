# Lumo 代码复查 — 核心问题修复方案

> 生成时间：2026-06-30  
> 复查范围：前端 Vue 3 / TypeScript / Pinia + 后端 Rust / Tauri / SQLite  
> 共 18 个问题，本文档针对 12 个核心问题给出详细修复方案

---

## 目录

- [🔴 P0-1: WebDAV 凭据明文存储](#p0-1-webdav-凭据明文存储)
- [🔴 P0-2: Gapless 播放与传统切歌竞态条件](#p0-2-gapless-播放与传统切歌竞态条件)
- [🔴 P0-3: progressMs watch 高频写 localStorage](#p0-3-progressms-watch-高频写-localstorage)
- [🟠 P1-1: Player Store 上帝对象（1600 行）](#p1-1-player-store-上帝对象1600-行)
- [🟠 P1-2: 前端大量 any 类型](#p1-2-前端大量-any-类型)
- [🟠 P1-3: Shuffle 模式 prevTrack 行为错误](#p1-3-shuffle-模式-prevtrack-行为错误)
- [🟠 P1-4: source_remove 缺少 artwork 孤儿清理](#p1-4-source_remove-缺少-artwork-孤儿清理)
- [🟠 P1-5: 缩略图回填 674 次独立事务](#p1-5-缩略图回填-674-次独立事务)
- [🟠 P1-6: chrono 已引入但手写日期算法](#p1-6-chrono-已引入但手写日期算法)
- [🟠 P1-7: WebDAV 路径替换可疑](#p1-7-webdav-路径替换可疑)
- [🟠 P1-8: Tauri listen 返回值未保存](#p1-8-tauri-listen-返回值未保存)
- [🟡 P2-1: 其他次要问题汇总](#p2-1-其他次要问题汇总)

---

## P0-1: WebDAV 凭据明文存储

### 问题描述

`src-tauri/src/commands/scanner.rs` 第 29-30 行：

```rust
let cred = if let (Some(u), Some(p)) = (&username, &password) {
    Some(format!("{}:{}", u, p))  // 明文 username:password
```

WebDAV 的用户名和密码以 `username:password` 的格式直接存入 SQLite 的 `credential_ref` 字段。任何能打开 `lumo.sqlite` 的人（包括恶意软件、备份同步工具等）都能直接读取密码。

### 影响范围

- `src-tauri/src/commands/scanner.rs` — `source_add_webdav` 写入
- `src-tauri/src/commands/scanner.rs` — `source_scan` 读取
- `src-tauri/src/commands/playback.rs` — `resolve_media_file` 读取

### 修复方案

**方案 A：使用操作系统钥匙串（推荐）**

利用 `keyring` crate（跨平台：Windows Credential Manager / macOS Keychain / Linux Secret Service）：

```toml
# Cargo.toml
keyring = "3"
```

```rust
// scanner.rs — 存储
fn store_webdav_credential(source_id: i64, username: &str, password: &str) -> Result<(), AppError> {
    let service = format!("lumo_webdav_{}", source_id);
    let entry = keyring::Entry::new(&service, username)
        .map_err(|e| AppError::Internal(format!("Keyring error: {}", e)))?;
    entry.set_password(password)
        .map_err(|e| AppError::Internal(format!("Keyring error: {}", e)))?;
    Ok(())
}

// scanner.rs — 读取
fn load_webdav_credential(source_id: i64, username: &str) -> Result<String, AppError> {
    let service = format!("lumo_webdav_{}", source_id);
    let entry = keyring::Entry::new(&service, username)
        .map_err(|e| AppError::Internal(format!("Keyring error: {}", e)))?;
    entry.get_password()
        .map_err(|e| AppError::Internal(format!("Keyring error: {}", e)))
}
```

数据库 `credential_ref` 字段改为只存用户名（作为钥匙串的查找键），密码存在 OS 钥匙串中。

**方案 B：最低限度——对称加密存储**

如果不想引入钥匙串依赖，至少用机器特定的密钥做 AES 加密：

```rust
// 用 app_data_dir 路径的 hash 作为加密密钥（不完美但好过明文）
fn encrypt_credential(plaintext: &str, app_dir: &Path) -> String { ... }
fn decrypt_credential(ciphertext: &str, app_dir: &Path) -> String { ... }
```

**数据迁移**：需要在 `apply_migrations` 中加一个 V6 迁移，将已有明文凭据迁移到新存储方式。

### 修改文件

| 文件 | 操作 |
|------|------|
| `Cargo.toml` | 添加 `keyring = "3"` |
| `src-tauri/src/commands/scanner.rs` | `source_add_webdav` 改用钥匙串存储 |
| `src-tauri/src/commands/scanner.rs` | `source_scan` 改用钥匙串读取 |
| `src-tauri/src/commands/playback.rs` | `resolve_media_file` 改用钥匙串读取 |
| `src-tauri/src/db.rs` | 添加 V6 迁移：已有凭据迁移 |

---

## P0-2: Gapless 播放与传统切歌竞态条件

### 问题描述

`src/stores/player.ts` 第 1280-1314 行，`startProgressPolling` 中存在两个切歌路径：

1. **Gapless 路径**（第 1280-1300 行）：检测到 `playbackGetQueueLen() === 1` 时静默切换
2. **传统兜底**（第 1302-1314 行）：检测到 `pos >= durationMs - 500` 或 `backendFinished` 时调用 `nextTrack(true)`

当 gapless 已预加载（`hasEnqueuedNext = true`），但后端队列尚未从 2→1 完成切换时：
- `queueLen` 仍然是 2 → gapless 路径不生效
- `reachedEnd` 为 true → 走传统兜底 → 调用 `nextTrack(true)` → 打断正在无缝播放的音频

### 修复方案

**核心原则**：当 gapless 已预加载，传统兜底必须跳过，完全依赖队列长度检测。

```typescript
// src/stores/player.ts — startProgressPolling 内部

// ===== 切歌判定 =====

// 路径 1: Gapless 模式 — 队列长度检测
if (hasEnqueuedNext) {
    try {
        const queueLen = await playbackGetQueueLen();
        if (queueLen === 1) {
            console.log("[Gapless] Silent transition to next track!");
            // ... 静默切换逻辑（现有代码不变）
        }
        // 无论 queueLen 是多少，都直接 return，
        // 不再走传统兜底，避免重复切歌
    } catch(e) {}
    return; // ← 关键：加在这里，跳过下面的传统兜底
}

// 路径 2: 传统兜底 — 仅在 gapless 未启用时执行
const reachedEnd = durationMs.value > 0 && pos >= durationMs.value - 500;
let backendFinished = false;
if (!reachedEnd) {
    try { backendFinished = await playbackIsFinished(); } catch {}
}
if (reachedEnd || backendFinished) {
    nextTrack(true);
}
```

**额外加固**：在 `nextTrack` 内部重置 gapless 状态，防止残留：

```typescript
async function nextTrack(isAuto = false) {
    // 重置 gapless 状态
    hasEnqueuedNext = false;
    enqueuedTrackIndex = null;
    // ... 现有逻辑
}
```

### 修改文件

| 文件 | 操作 |
|------|------|
| `src/stores/player.ts` | `startProgressPolling` — gapless 检测后加 `return` |
| `src/stores/player.ts` | `nextTrack` — 入口重置 gapless 状态 |

---

## P0-3: progressMs watch 高频写 localStorage

### 问题描述

`src/stores/player.ts` 第 1119-1121 行：

```typescript
watch(progressMs, (newProgress) => {
    localStorage.setItem('lumo_progress_ms', String(newProgress));
});
```

`progressMs` 每 500ms 由 `startProgressPolling` 更新，每次更新触发 `watch` → `localStorage.setItem`（同步磁盘写入），即每秒 2 次。

### 修复方案

**方案：移除 watch，改为事件驱动保存**

删除 `watch(progressMs, ...)`，改为在以下时机手动保存：

```typescript
// 1. 暂停时保存
async function togglePlay() {
    // ...
    if (isPlaying.value) {
        await playbackPause();
        isPlaying.value = false;
        saveProgressToStorage(); // ← 暂停时保存
    }
    // ...
}

// 2. 切歌时保存（playQueue 已隐含保存 currentIndex）
// 3. 窗口关闭前保存
function saveProgressToStorage() {
    localStorage.setItem('lumo_progress_ms', String(progressMs.value));
}

// 在 App.vue 的 onMounted 中注册
window.addEventListener('beforeunload', saveProgressToStorage);
```

如果仍需定期保存（防止异常退出丢失进度），可降频为每 30 秒一次：

```typescript
let progressSaveTimer: ReturnType<typeof setInterval> | null = null;

// 开始播放时启动
function startProgressAutoSave() {
    if (progressSaveTimer) return;
    progressSaveTimer = setInterval(saveProgressToStorage, 30000); // 30 秒
}

// 暂停/停止时清除
function stopProgressAutoSave() {
    if (progressSaveTimer) {
        clearInterval(progressSaveTimer);
        progressSaveTimer = null;
    }
    saveProgressToStorage(); // 停止时立即保存一次
}
```

### 修改文件

| 文件 | 操作 |
|------|------|
| `src/stores/player.ts` | 删除 `watch(progressMs, ...)`，添加 `saveProgressToStorage()` |
| `src/stores/player.ts` | `togglePlay` / `playQueue` / `nextTrack` 中调用保存 |
| `src/App.vue` | `onMounted` 中注册 `beforeunload` |

---

## P1-1: Player Store 上帝对象（1600 行）

### 问题描述

`src/stores/player.ts` 承担了 10+ 种职责，1600 行代码全部在一个 `defineStore` 中。

### 修复方案

**拆分为 5 个独立 Store**，通过 Pinia 的 `useXxxStore()` 互相引用：

```
src/stores/
├── playback.ts      ← 播放控制 (play/pause/seek/volume/queue/gapless)
├── library.ts       ← 库数据 (tracks/albums/artists 分页加载)
├── playlist.ts      ← 歌单管理 (create/delete/add/remove)
├── navigation.ts    ← 导航状态 (activeTab/history/back/forward)
└── folder.ts        ← 文件夹浏览 (tree/contents/pagination)
```

**拆分规则**：

| 新 Store | 从 player.ts 迁出的状态和方法 |
|----------|-------------------------------|
| `playback` | `isPlaying`, `volume`, `queue`, `currentIndex`, `playMode`, `progressMs`, `durationMs`, `currentTrack`, `togglePlay`, `playQueue`, `nextTrack`, `prevTrack`, `seek`, `setVolume`, `startProgressPolling`, gapless 相关, `restoreSession`, `persistPlayQueueIfNeeded`, `lyrics`, `activeLyricIndex`, `parseLrc` |
| `library` | `tracks`, `albums`, `artists`, `fetchTracks`, `fetchAlbums`, `fetchArtists`, `fetchCounts`, `toggleFavorite`, `favoriteAlbums`, `favoriteArtists`, `searchQuery`, `currentAlbumDetailsData`, `currentArtistDetailsData`, `sources`, `addSource`, `removeSource`, `scanSource` |
| `playlist` | `playlists`, `fetchPlaylists`, `createPlaylist`, `deletePlaylist`, `addToPlaylist`, `removeTrackFromPlaylist`, `refreshCurrentPlaylistTracks`, `isCreatePlaylistModalOpen` |
| `navigation` | `activeLibraryTab`, `activeSourceTab`, `activeRightTab`, `activeAlbumId`, `activeArtistId`, `activePlaylistId`, `globalSearchQuery`, `historyStack`, `forwardStack`, `goBack`, `goForward`, `canGoBack`, `canGoForward` |
| `folder` | `currentFolderContents`, `activeFolderSourceId`, `activeFolderPath`, `folderBreadcrumbs`, `fetchFolderContents`, `fetchMoreFolderEntries`, `folderTreeChildren`, `fetchFolderTreeChildren`, `fetchFolderTracks` |

**迁移步骤**：

1. 从最独立的 `folder.ts` 开始拆分（与其他状态依赖最少）
2. 拆分 `navigation.ts`（只依赖 tab/id 状态）
3. 拆分 `playlist.ts`
4. 最后拆分 `playback.ts` 和 `library.ts`（互相引用最多）
5. 保留 `player.ts` 作为门面（facade），re-export 所有 store

**兼容性**：现有组件中 `usePlayerStore()` 不用立即全部改，可以在 `player.ts` 中 re-export：

```typescript
// src/stores/player.ts（门面）
export const usePlayerStore = defineStore("player", () => {
    const playback = usePlaybackStore();
    const library = useLibraryStore();
    // ... 把所有属性和方法 spread 出去，保持向后兼容
    return { ...playback, ...library, ... };
});
```

### 修改文件

| 文件 | 操作 |
|------|------|
| `src/stores/playback.ts` | [NEW] 播放控制 store |
| `src/stores/library.ts` | [NEW] 库数据 store |
| `src/stores/playlist.ts` | [NEW] 歌单 store |
| `src/stores/navigation.ts` | [NEW] 导航 store |
| `src/stores/folder.ts` | [NEW] 文件夹 store |
| `src/stores/player.ts` | [MODIFY] 改为门面，re-export |

---

## P1-2: 前端大量 any 类型

### 问题描述

6+ 处使用 `any` 类型，后端有完整 DTO 结构体但前端缺少对应接口。

### 修复方案

在 `src/api/types.ts` 中补齐缺失的接口：

```typescript
// src/api/types.ts — 新增

export interface ArtistStatsDTO {
    track_count: number;
    album_count: number;
}

export interface TrackFileInfoDTO {
    id: number;
    path: string;
    file_size: number | null;
    duration_ms: number | null;
    bitrate: number | null;
    sample_rate: number | null;
    bit_depth: number | null;
    channels: number | null;
    format: string | null;
}

export interface SourceDTO {
    id: number;
    name: string;
    kind: string;
    root_uri: string;
    config_json: string;
    credential_ref: string | null;
    enabled: boolean;
    last_scan_at: string | null;
    last_error: string | null;
    created_at: string;
    updated_at: string;
}
```

然后更新 API 函数返回类型：

```typescript
// src/api/library.ts
export function libraryGetArtistStats(artistId: number): Promise<ArtistStatsDTO> { ... }
export function libraryGetTrackFileInfo(trackId: number): Promise<TrackFileInfoDTO | null> { ... }

// src/api/scanner.ts
export function sourceList(): Promise<SourceDTO[]> { ... }
```

在 `src/stores/player.ts` 中替换 `ref<any>` 为具体类型：

```typescript
// 定义详情页接口
interface AlbumDetails extends Album {
    tracks: Track[];
}

interface ArtistDetails extends Artist {
    stats: ArtistStatsDTO;
    tracks: Track[];
    albums: Album[];
    tracksOffset: number;
    hasMoreTracks: boolean;
    isLoadingTracks: boolean;
    albumsCurrentPage: number;
    albumsTotalCount: number;
    albumsTotalPages: number;
    hasMoreAlbums: boolean;
    isLoadingAlbums: boolean;
}

interface PlaylistDetails extends Playlist {
    tracks: Track[];
    isLoadingTracks: boolean;
}

// 替换
const currentAlbumDetailsData = ref<AlbumDetails | null>(null);
const currentArtistDetailsData = ref<ArtistDetails | null>(null);
const currentPlaylistDetailsData = ref<PlaylistDetails | null>(null);
const currentTrackFileInfo = ref<TrackFileInfoDTO | null>(null);
const lyrics = ref<LyricLine[]>([]);
```

### 修改文件

| 文件 | 操作 |
|------|------|
| `src/api/types.ts` | 添加 `ArtistStatsDTO`, `TrackFileInfoDTO`, `SourceDTO` |
| `src/api/library.ts` | 更新返回类型 |
| `src/api/scanner.ts` | 更新返回类型 |
| `src/stores/player.ts` | 替换所有 `ref<any>` |

---

## P1-3: Shuffle 模式 prevTrack 行为错误

### 问题描述

`src/stores/player.ts` 第 1364-1372 行，shuffle 模式下 `prevTrack` 随机跳转而非回退：

```typescript
if (playMode.value === 'shuffle') {
    currentIndex.value = Math.floor(Math.random() * queue.value.length);
}
```

### 修复方案

引入播放历史栈，`prevTrack` 从历史栈弹出：

```typescript
// 播放历史栈（仅记录 index，不记录整个 track 对象）
const playHistory: number[] = [];
const MAX_PLAY_HISTORY = 100;

// 在 playQueue 中记录历史
async function playQueue(newQueue: Track[], index: number) {
    // 切歌前把当前 index 压入历史
    if (currentIndex.value >= 0 && currentIndex.value < queue.value.length) {
        playHistory.push(currentIndex.value);
        if (playHistory.length > MAX_PLAY_HISTORY) {
            playHistory.shift();
        }
    }
    // ... 现有逻辑
}

// prevTrack 从历史栈弹出
async function prevTrack() {
    if (queue.value.length === 0) return;
    
    if (playMode.value === 'shuffle' && playHistory.length > 0) {
        // Shuffle 模式：回退到上一首播放过的歌
        currentIndex.value = playHistory.pop()!;
    } else if (playMode.value === 'shuffle') {
        // Shuffle 模式但无历史：随机（兜底）
        currentIndex.value = Math.floor(Math.random() * queue.value.length);
    } else {
        // 顺序模式：往前一首
        currentIndex.value = (currentIndex.value - 1 + queue.value.length) % queue.value.length;
    }
    
    await playQueue(queue.value, currentIndex.value);
}
```

注意 `playQueue` 内部调用时不要把当前 index 重复压栈，需要加一个 `skipHistoryPush` 参数或在 `prevTrack` 中临时标记。

### 修改文件

| 文件 | 操作 |
|------|------|
| `src/stores/player.ts` | 添加 `playHistory` 栈 |
| `src/stores/player.ts` | `playQueue` 中压栈 |
| `src/stores/player.ts` | `prevTrack` 改为弹栈 |

---

## P1-4: source_remove 缺少 artwork 孤儿清理

### 问题描述

`src-tauri/src/commands/scanner.rs` 的 `source_remove` 事务中清理了 `tracks`、`albums`、`artists` 的孤儿记录，但遗漏了 `artwork` 表和磁盘缓存。

### 修复方案

在事务中添加 artwork 清理步骤：

```rust
// src-tauri/src/commands/scanner.rs — source_remove 事务内

// 现有的 DELETE tracks / albums / artists ... 之后添加：

// 清理无主的 artwork 记录
// artwork 通过 media_file_id 关联 media_files，
// media_files 已通过 ON DELETE CASCADE 被删除，
// 但 artwork.media_file_id 是 SET NULL，所以 artwork 行本身还在。
// 需要主动清理引用计数为 0 的 artwork。
let orphan_artworks: Vec<(i64, String)> = {
    let mut stmt = tx.prepare(
        "SELECT a.id, a.cache_path FROM artwork a
         WHERE a.id NOT IN (
             SELECT cover_artwork_id FROM albums WHERE cover_artwork_id IS NOT NULL
         )"
    )?;
    stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect()
};

for (art_id, cache_path) in &orphan_artworks {
    tx.execute("DELETE FROM artwork WHERE id = ?1", params![art_id])?;
    // 删除磁盘缓存文件（锁外做更好，但在事务内删少量文件可接受）
    let _ = std::fs::remove_file(cache_path);
}
```

### 修改文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/commands/scanner.rs` | `source_remove` 事务内添加 artwork 清理 |

---

## P1-5: 缩略图回填 674 次独立事务

### 问题描述

`src-tauri/src/lib.rs` 的 `backfill_artwork_thumbnails` 中每张图片单独 `pool.get()` + `execute`，674 张图片 = 674 个 SQLite 事务。

### 修复方案

改为批量事务，每 50 张提交一次：

```rust
fn backfill_artwork_thumbnails(app: tauri::AppHandle, pool: &DbPool) {
    // ... 查询 rows 的代码不变 ...
    
    let batch_size = 50;
    let mut done = 0usize;
    let mut failed = 0usize;
    
    // 按批次处理
    for chunk in rows.chunks(batch_size) {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("[回填] 获取数据库连接失败: {}", e);
                break;
            }
        };
        
        // 一个事务处理一批
        let tx = match conn.unchecked_transaction() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("[回填] 开启事务失败: {}", e);
                break;
            }
        };
        
        for (id, cache_path) in chunk {
            let thumb = std::fs::read(cache_path)
                .ok()
                .and_then(|data| crate::services::library::LibraryService::generate_thumbnail(&data));
            
            if let Some(blob) = thumb {
                let _ = tx.execute(
                    "UPDATE artwork SET thumbnail_blob = ?1 WHERE id = ?2",
                    params![blob, id],
                );
                done += 1;
            } else {
                failed += 1;
            }
        }
        
        let _ = tx.commit();
        
        // 每批完成后暂停，给系统喘息
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        if (done + failed) % 100 == 0 {
            tracing::info!("[回填] 进度：{}/{}（成功 {}，失败 {}）",
                done + failed, rows.len(), done, failed);
        }
    }
    
    // ... 完成通知 ...
}
```

### 修改文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/lib.rs` | `backfill_artwork_thumbnails` 改为批量事务 |

---

## P1-6: chrono 已引入但手写日期算法

### 问题描述

`Cargo.toml` 已引入 `chrono = "0.4.45"`，但 `lib.rs` 的 `format_http_date` 手写了完整的 civil_from_days 日期算法（40 行），注释说"为了避免引入 chrono"。

### 修复方案

**方案 A（推荐）：使用已有的 chrono**

```rust
// lib.rs
fn format_http_date(unix_secs: u64) -> Option<String> {
    use chrono::{DateTime, Utc};
    let dt = DateTime::from_timestamp(unix_secs as i64, 0)?;
    Some(dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
}
```

从 40 行减少到 4 行，且经过充分测试的库代码更可靠。

**方案 B：移除 chrono 依赖**

如果确实想减小二进制体积，确认 chrono 在其他地方是否被使用。如果只有这一处，可以保留手写实现并移除 `chrono` 依赖。

### 修改文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/lib.rs` | `format_http_date` 改用 `chrono` |
| 或 `src-tauri/Cargo.toml` | 移除 `chrono` 依赖（若其他地方未使用） |

---

## P1-7: WebDAV 路径替换可疑

### 问题描述

`src-tauri/src/commands/playback.rs` 第 39 行：

```rust
let relative_url_path = relative_path.replace("\\\\", "/");
```

Rust 字符串中 `"\\\\"` 代表两个反斜杠字符 `\\`。但 Windows 路径通常是单个反斜杠 `\`（如 `music\album\song.mp3`），不是 `\\`。

### 修复方案

应改为替换单个反斜杠：

```rust
let relative_url_path = relative_path.replace('\\', "/");
```

这会把 `music\album\song.mp3` 正确转换为 `music/album/song.mp3`。

需要同时验证数据库中 `media_files.relative_path` 实际存储的路径分隔符格式。如果扫描时已经归一化为 `/`，那这行替换可能根本不需要。

### 修改文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/commands/playback.rs` | 修改路径替换逻辑 |

---

## P1-8: Tauri listen 返回值未保存

### 问题描述

`src/stores/player.ts` 第 1461-1487 行注册了 3 个全局事件监听器，但返回的 `Promise<UnlistenFn>` 被丢弃。Vite HMR 期间会导致监听器重复注册。

### 修复方案

保存 unlisten 函数，在 store 初始化时清理旧监听：

```typescript
// 在 store 定义的顶层
let unlistenScanProgress: (() => void) | null = null;
let unlistenScanComplete: (() => void) | null = null;
let unlistenArtworkBackfill: (() => void) | null = null;

// 封装为异步初始化函数
async function initEventListeners() {
    // 先清理可能存在的旧监听（HMR 场景）
    unlistenScanProgress?.();
    unlistenScanComplete?.();
    unlistenArtworkBackfill?.();
    
    unlistenScanProgress = await listen('scan-progress', (event: any) => {
        // ... 现有逻辑
    });
    
    unlistenScanComplete = await listen('scan-complete', async (event: any) => {
        // ... 现有逻辑
    });
    
    unlistenArtworkBackfill = await listen('artwork-backfill-complete', async () => {
        // ... 现有逻辑
    });
}

// 在 store 的 return 之前调用
initEventListeners();
```

### 修改文件

| 文件 | 操作 |
|------|------|
| `src/stores/player.ts` | 保存 unlisten 引用，HMR 时清理 |

---

## P2-1: 其他次要问题汇总

| 编号 | 问题 | 修复方案 | 工作量 |
|------|------|----------|--------|
| P2-a | 专辑默认年份 `new Date().getFullYear()` | 改为 `0`，UI 层显示 `year > 0 ? year : '未知年份'` | 5 分钟 |
| P2-b | `playback_play` 持锁期间做 I/O | 将文件打开/解码器创建移到锁外，锁内只做 `sink.stop()` + `sink.append()` | 30 分钟 |
| P2-c | `library_get_counts` 5 个独立查询 | 合并为一个 `SELECT (子查询), (子查询), ...` | 10 分钟 |
| P2-d | LRU 淘汰 O(n) 遍历 | 用 Map 的插入顺序特性（删除再插入 = 移到末尾）简化，或引入双向链表 | 1 小时 |
| P2-e | `library.rs` 大量空行 | 删除第 12-22 行的 11 行空行 | 1 分钟 |
| P2-f | `ThemeKey` 类型未使用 | 删除 `ui.ts` 第 4 行，或在 `setDarkMode` 的参数中使用它 | 1 分钟 |

---

## 建议修复优先级

```
第一批（安全+正确性）：P0-1, P0-2, P0-3
第二批（代码质量）：    P1-2, P1-3, P1-7, P1-8
第三批（架构优化）：    P1-1, P1-4, P1-5, P1-6
第四批（打磨）：        P2-a ~ P2-f
```
