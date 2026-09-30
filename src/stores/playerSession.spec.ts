import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { flushPromises } from "@vue/test-utils";

/**
 * DM-02 常驻会话验收测试（desktop-modes 02 §4 DM-02）。
 *
 * 覆盖可自动化验收：
 *  - 验收 4：会话接线幂等——重复 ensureSessionWired 不新增监听，
 *    模式转换不允许重复注册（监听数量稳定）；
 *  - 验收 5：轻量摘要带版本守卫，迟到的旧响应不覆盖新会话；
 *  - restoreSession 只执行一次：模式转换不得重复恢复或重建 sink。
 *
 * IPC 走 src/utils/tauriInvoke.ts 咽喉点；事件走 @tauri-apps/api/event mock。
 */

const { invokeMock, eventHandlers, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  eventHandlers: new Map<string, (event: { payload: unknown }) => unknown>(),
  listenMock: vi.fn(),
}));

vi.mock("../utils/tauriInvoke", () => ({ invoke: invokeMock }));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, handler: (e: { payload: unknown }) => unknown) => {
    listenMock(event, handler);
    eventHandlers.set(event, handler);
    return () => {
      eventHandlers.delete(event);
    };
  }),
}));

import { usePlayerStore } from "./player";

/** 轻量会话摘要样本（与 Rust PlaybackSessionSummaryDto 对应） */
function summary(overrides: Record<string, unknown> = {}) {
  return {
    stateVersion: 1,
    queueLength: 3,
    currentIndex: 0,
    mode: "normal",
    positionMs: 0,
    durationMs: 180000,
    isPlaying: false,
    currentTrack: {
      trackId: 1, mediaFileId: 101, title: "曲目一", artist: "歌手",
      album: "专辑", artworkId: null, durationMs: 180000,
    },
    ...overrides,
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
  localStorage.clear();
  invokeMock.mockReset();
  listenMock.mockClear();
  eventHandlers.clear();
});

describe("DM-02 常驻会话", () => {
  it("验收4：会话接线幂等，重复调用不新增任何监听", async () => {
    invokeMock.mockImplementation(() => Promise.resolve([]));
    const store = usePlayerStore();

    // store 创建时接线一次；initEventListeners 为异步注册，等微任务刷完
    await vi.waitFor(() => expect(listenMock.mock.calls.length).toBeGreaterThanOrEqual(9));
    const afterInit = listenMock.mock.calls.length;
    expect(store.sessionWiredCount).toBe(1);

    // 模式转换场景：反复调用接线入口，不产生第二个监听
    for (let i = 0; i < 20; i++) {
      store.ensureSessionWired();
    }
    expect(store.sessionWiredCount).toBe(1);
    expect(listenMock.mock.calls.length).toBe(afterInit);
  });

  it("验收5：轻量摘要按版本守卫应用，迟到的旧响应不覆盖新会话", async () => {
    const summaries: Record<number, unknown> = {
      5: summary({ stateVersion: 5, currentIndex: 2, positionMs: 99_000, isPlaying: true }),
      3: summary({ stateVersion: 3, currentIndex: 0, positionMs: 1_000, isPlaying: false }),
    };
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "playback_session_summary") {
        // 依次返回「旧版本」再返回「新版本」——模拟乱序到达
        return Promise.resolve(summaries[pendingOrder.shift() ?? 5]);
      }
      return Promise.resolve([]);
    });
    const pendingOrder = [3, 5];

    const store = usePlayerStore();
    await store.syncSessionFromBackend();
    // 镜像为空：index 不越界写入（保持初始 -1），标量按摘要对账
    expect(store.currentIndex).toBe(-1);
    expect(store.progressMs).toBe(1000);
    expect(store.lastSessionSummary?.stateVersion).toBe(3);

    await store.syncSessionFromBackend();
    expect(store.currentIndex).toBe(-1);
    expect(store.progressMs).toBe(99000);
    expect(store.isPlaying).toBe(true);
    expect(store.lastSessionSummary?.stateVersion).toBe(5);

    // 迟到的 v3 再到达：必须被丢弃，不覆盖 v5 的会话状态
    pendingOrder.push(3);
    await store.syncSessionFromBackend();
    expect(store.lastSessionSummary?.stateVersion).toBe(5);
    expect(store.progressMs).toBe(99000);
    expect(store.isPlaying).toBe(true);
  });

  it("restoreSession 只执行一次：重复调用不重复恢复队列与音量", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "library_get_play_queue") {
        return Promise.resolve([
          { id: 1, title: "曲目一", artist: "歌手", album: "专辑", duration: 180, durationSec: 180, format: "MP3", cover_artwork_id: null, isFavorite: false },
        ]);
      }
      if (cmd === "library_get_counts") {
        return Promise.resolve({ tracks: 1, albums: 1, artists: 1, favorites: 0, playlists: 0, sources: 1 });
      }
      return Promise.resolve(null);
    });

    const store = usePlayerStore();
    await store.restoreSession(null);
    const callsAfterFirst = invokeMock.mock.calls.length;
    expect(store.queue).toHaveLength(1);

    // 模式转换误触的第二次恢复：no-op，不产生额外 IPC
    await store.restoreSession(null);
    await store.restoreSession(null);
    expect(invokeMock.mock.calls.length).toBe(callsAfterFirst);
  });

  it("轻量摘要只同步会话标量，不重建浏览队列镜像", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "playback_session_summary") {
        return Promise.resolve(summary({ stateVersion: 9, queueLength: 30000, currentIndex: 1 }));
      }
      return Promise.resolve([]);
    });

    const store = usePlayerStore();
    await store.syncSessionFromBackend();

    // 队列镜像保持为空（30k 队列不复制）；标量按摘要对账
    expect(store.queue).toHaveLength(0);
    expect(store.lastSessionSummary?.queueLength).toBe(30000);
    expect(store.currentIndex).toBe(-1); // 镜像为空时不强行写入越界 index
    expect(store.playMode).toBe("normal");
  });

  it("DM-04 验收3：极简下切歌不自动加载，显式打开歌词按需加载且去重", async () => {
    localStorage.setItem("lumo_fetch_lyrics", "1");
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "library_get_lyrics") return Promise.resolve("[00:01.00]测试歌词行");
      if (cmd === "library_get_track_file_info") return Promise.resolve(null);
      return Promise.resolve([]);
    });

    const { useDesktopModeStore } = await import("./desktopMode");
    const desktopMode = useDesktopModeStore();
    await desktopMode.init();
    await desktopMode.setExperienceMode("minimal");

    const store = usePlayerStore();
    store.queue = [{
      id: 7, title: "曲目七", artistId: null, artist: "歌手", albumId: null,
      album: "专辑", duration: "3:00", durationSec: 180, format: "MP3",
      coverColor: "", cover_artwork_id: null, isFavorite: false,
      primary_file_id: 107, fileSize: null, sourceKind: "local",
    }] as never;
    store.currentIndex = 0;
    await new Promise((r) => setTimeout(r, 250));
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === "library_get_lyrics")).toHaveLength(0);

    // 显式打开（LyricsView 挂载）：按授权加载，歌词写入
    await store.ensureLyricsLoaded();
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === "library_get_lyrics")).toHaveLength(1);
    expect(store.lyrics.length).toBeGreaterThan(0);

    // 再次调用（视图重挂载等）幂等：同一首歌不重复请求
    await store.ensureLyricsLoaded();
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === "library_get_lyrics")).toHaveLength(1);
  });

  it("DM-03：极简体验下切歌不加载歌词，正常体验恢复自动加载", async () => {
    localStorage.setItem("lumo_fetch_lyrics", "1"); // 联网歌词授权开启（偏好不变性由 DM-01 验收覆盖）
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "library_get_lyrics") return Promise.resolve(null);
      if (cmd === "library_get_track_file_info") return Promise.resolve(null);
      return Promise.resolve([]);
    });

    // 体验切到极简：visualAllowed=false
    const { useDesktopModeStore } = await import("./desktopMode");
    const desktopMode = useDesktopModeStore();
    await desktopMode.init();
    await desktopMode.setExperienceMode("minimal");
    expect(desktopMode.visualAllowed).toBe(false);

    const store = usePlayerStore();
    store.queue = [{
      id: 1, title: "曲目一", artistId: null, artist: "歌手", albumId: null,
      album: "专辑", duration: "3:00", durationSec: 180, format: "MP3",
      coverColor: "", cover_artwork_id: null, isFavorite: false,
      primary_file_id: 101, fileSize: null, sourceKind: "local",
    }] as never;
    store.currentIndex = 0;
    // 歌词加载有 100ms 防抖
    await new Promise((r) => setTimeout(r, 250));

    const lyricCalls = invokeMock.mock.calls.filter(([cmd]) => cmd === "library_get_lyrics");
    expect(lyricCalls).toHaveLength(0);

    // 切回正常：同一首歌重新触发自动加载
    await desktopMode.setExperienceMode("normal");
    store.currentIndex = -1;
    await nextTick(); // 先让 computed 冲刷到 undefined，再切回触发 watch
    store.currentIndex = 0;
    await new Promise((r) => setTimeout(r, 250));
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === "library_get_lyrics").length).toBeGreaterThan(0);
  });

  it("DM-05 验收1/3：进入迷你释放浏览数组，返回按锚点窗口恢复且不逐页回放", async () => {
    const trackDto = (i: number) => ({
      id: i, title: `曲目${i}`, artist: "歌手", artist_id: null, album: "专辑", album_id: null,
      duration_ms: 180000, file_ext: "MP3", media_file_id: i + 1000, is_favorite: false,
      cover_artwork_id: null, file_size: null, source_kind: "local", year: null,
      genres: null, bitrate: null, sample_rate: null, bit_depth: null,
    });
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "library_get_tracks") {
        const offset = (args?.offset as number) ?? 0;
        if ((args?.limit as number) === 200) {
          // 播种请求（limit=200）：以锚点偏移为起点返回 200 行窗口
          return Promise.resolve(Array.from({ length: 200 }, (_, k) => trackDto(offset + k)));
        }
        if (offset === 0) return Promise.resolve(Array.from({ length: 50 }, (_, k) => trackDto(k)));
        if (offset === 50) return Promise.resolve(Array.from({ length: 50 }, (_, k) => trackDto(k + 50)));
        if (offset === 100) return Promise.resolve(Array.from({ length: 50 }, (_, k) => trackDto(k + 100)));
        return Promise.resolve([]);
      }
      if (cmd === "desktop_get_preferences") {
        return Promise.resolve({
          preferences: { schemaVersion: 1, experienceMode: "normal", windowForm: "full", fullGeometry: null, miniGeometry: null, miniAlwaysOnTop: false },
          fileExisted: true,
        });
      }
      return Promise.resolve([]);
    });

    const { useDesktopModeStore } = await import("./desktopMode");
    const desktopMode = useDesktopModeStore();
    await desktopMode.init();

    const store = usePlayerStore();
    // 模拟核心会话与深页浏览（3 页 → offset=150）
    store.activeLibraryTab = "全部歌曲";
    store.queue = [{
      id: 999, title: "播放中", artistId: null, artist: "歌手", albumId: null,
      album: "专辑", duration: "3:00", durationSec: 180, format: "MP3",
      coverColor: "", cover_artwork_id: null, isFavorite: false,
      primary_file_id: 1, fileSize: null, sourceKind: "local",
    }] as never;
    store.currentIndex = 0;
    await store.fetchTracks();
    await store.fetchTracks();
    await store.fetchTracks();
    expect(store.tracks).toHaveLength(150);

    // 进入迷你：浏览数组释放，核心会话保留
    expect(await desktopMode.enterMini()).toBe(true);
    expect(store.tracks).toHaveLength(0);
    expect(store.albums).toHaveLength(0);
    expect(store.playlists).toHaveLength(0);
    expect(store.lyrics).toHaveLength(0);
    expect(store.queue).toHaveLength(1); // 核心会话不动

    // 返回完整：挂载恢复待办 → 锚点窗口播种（offset=100，不是从 0 逐页）
    expect(await desktopMode.exitMini()).toBe(true);
    expect(store.pendingBrowseRestore).toBe(true);
    await store.applyBrowseRestore();
    const seedCall = invokeMock.mock.calls.find(([cmd, args]) =>
      cmd === "library_get_tracks" && (args as Record<string, unknown>)?.offset === 100);
    expect(seedCall).toBeTruthy();
    expect(store.tracks).toHaveLength(200); // 窗口整体替换，非 0+50+50+… 回放
    expect(store.pendingBrowseRestore).toBe(false);
  });

  it("DM-05 验收2：非视觉态扫描完成不拉完整曲库，恢复视觉态合并刷新一次", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "library_get_counts") {
        return Promise.resolve({ tracks: 32000, albums: 3000, artists: 2000, favorites: 0, playlists: 0, sources: 1, favorite_tracks: 0, favorite_albums: 0, favorite_artists: 0, recently_played: 0 });
      }
      return Promise.resolve([]);
    });

    const { useDesktopModeStore } = await import("./desktopMode");
    const desktopMode = useDesktopModeStore();
    await desktopMode.init();
    await desktopMode.setExperienceMode("minimal");

    const store = usePlayerStore();
    await vi.waitFor(() => expect(eventHandlers.has("scan-complete")).toBe(true));

    const bigFetchesBefore = invokeMock.mock.calls.filter(([cmd]) =>
      ["library_get_tracks", "library_get_albums", "library_get_artists"].includes(cmd as string)).length;
    eventHandlers.get("scan-complete")!({ payload: { source_id: 1, success: true } });
    await flushPromises();
    eventHandlers.get("scan-complete")!({ payload: { source_id: 1, success: true } });
    await flushPromises();

    // 两次扫描完成：完整曲库零拉取，仅计数 + dirty
    const bigFetchesAfter = invokeMock.mock.calls.filter(([cmd]) =>
      ["library_get_tracks", "library_get_albums", "library_get_artists"].includes(cmd as string)).length;
    expect(bigFetchesAfter).toBe(bigFetchesBefore);
    expect(store.browseDirty).toBe(true);

    // 恢复视觉态：合并刷新一次（tracks/albums/artists 各一次 reset 拉取）
    await desktopMode.setExperienceMode("normal");
    await flushPromises();
    const tracksReset = invokeMock.mock.calls.filter(([cmd, args]) =>
      cmd === "library_get_tracks" && (args as Record<string, unknown>)?.offset === 0);
    expect(tracksReset.length).toBeGreaterThan(0);
    expect(store.browseDirty).toBe(false);
  });

  it("DM-05 验收4：结果集级全选覆盖完整结果集，手工切换退出该语义", async () => {
    const { useBatchSelect } = await import("../composables/useBatchSelect");
    const batch = useBatchSelect();
    batch.enter();
    batch.selectAllIds([11, 12, 13, 14]);
    expect(batch.count).toBe(4);
    expect(batch.resultWide).toBe(true);
    expect(batch.isSelected(12)).toBe(true);
    expect(batch.isSelected(99)).toBe(false);

    batch.toggle({ id: 12 } as never);
    expect(batch.resultWide).toBe(false); // 手工切换退出结果集级语义

    batch.selectAllIds([]);
    expect(batch.resultWide).toBe(false);
    expect(batch.count).toBe(0);
  });
});
