import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { SourceDTO } from "../api/types";

/**
 * player store 的来源（source）状态测试 —— 对应 I3/G-08。
 *
 * 被验证的行为：后端把"上次扫描失败原因"存在 sources.last_error，且只在扫描
 * 成功时推进 last_scan_at。前端必须
 *   1. 把 last_error 映射进 MusicSource.lastError；
 *   2. 扫描结束时重新拉取后端状态，而不是写死"刚刚扫描"。
 *
 * 全部 IPC 走 src/utils/tauriInvoke.ts 这一个咽喉点，所以只需 mock 它，
 * api 包装层与 store 的真实映射逻辑都会被执行。
 */

const { invokeMock, eventHandlers } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  eventHandlers: new Map<string, (event: { payload: unknown }) => unknown>(),
}));

vi.mock("../utils/tauriInvoke", () => ({ invoke: invokeMock }));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, handler: (e: { payload: unknown }) => unknown) => {
    eventHandlers.set(event, handler);
    return () => {
      eventHandlers.delete(event);
    };
  }),
}));

function makeSource(overrides: Partial<SourceDTO> & { id: number }): SourceDTO {
  return {
    name: `来源 ${overrides.id}`,
    kind: "local",
    root_uri: `D:/Music${overrides.id}`,
    config_json: "{}",
    username: null,
    enabled: true,
    last_scan_at: null,
    last_error: null,
    created_at: "2026-09-01 00:00:00",
    updated_at: "2026-09-01 00:00:00",
    ...overrides,
  };
}

/**
 * source_list 返回指定来源；scan-complete 会连带重拉的命令返回形状正确的空结果，
 * 避免"mock 形状不对"掩盖成被测代码的真实缺陷。
 */
const EMPTY_BY_COMMAND: Record<string, unknown> = {
  library_get_tracks: [],
  library_get_albums: [],
  library_get_album_count: 0,
  // 后端把艺人列表包了一层（{ artists, total }），fetchArtists 会解构它
  library_get_artists: { artists: [], total: 0 },
};

function stubSourceList(sources: SourceDTO[], options: { failSourceList?: () => boolean } = {}) {
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd !== "source_list") return EMPTY_BY_COMMAND[cmd] ?? [];
    if (options.failSourceList?.()) throw new Error("IPC 通道拥堵");
    return sources;
  });
}

describe("playerStore.fetchSources 映射 last_error", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    eventHandlers.clear();
  });

  it("把 last_error 透出到 lastError，并把 last_scan_at 本地化", async () => {
    const { usePlayerStore } = await import("../stores/player");
    stubSourceList([
      makeSource({
        id: 1,
        last_scan_at: "2026-09-18T02:00:00Z",
        last_error: "WebDAV 连接超时",
      }),
      makeSource({ id: 2, last_scan_at: "2026-09-18T03:00:00Z" }),
      makeSource({ id: 3 }),
    ]);

    const store = usePlayerStore();
    await store.fetchSources();

    const [a, b, c] = store.sources;
    expect(a.lastError).toBe("WebDAV 连接超时");
    // 有失败原因时时间戳仍是"上次成功"的时刻，两者必须同时呈现
    expect(a.lastScanned).toBe(new Date("2026-09-18T02:00:00Z").toLocaleString());
    expect(b.lastError).toBeUndefined();
    expect(c.lastScanned).toBe("Never");
    expect(c.lastError).toBeUndefined();
  });

  it("source_list 失败时保留原有列表，不把来源清空", async () => {
    const { usePlayerStore } = await import("../stores/player");
    let failing = false;
    const list = [makeSource({ id: 1 })];
    stubSourceList(list, { failSourceList: () => failing });
    const store = usePlayerStore();
    await store.fetchSources();
    expect(store.sources).toHaveLength(1);

    failing = true;
    await store.fetchSources();
    expect(store.sources).toHaveLength(1);
  });
});

describe("scan-complete 事件后的状态刷新", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    eventHandlers.clear();
  });

  it("重新拉取后端状态：失败的扫描不会显示成刚刚扫描成功", async () => {
    const { usePlayerStore } = await import("../stores/player");
    const successAt = "2026-09-18T02:00:00Z";
    stubSourceList([makeSource({ id: 1, last_scan_at: successAt })]);

    const store = usePlayerStore();
    await store.fetchSources();
    await vi.waitFor(() => {
      expect(eventHandlers.has("scan-complete")).toBe(true);
    });

    // 本轮扫描失败：last_scan_at 不动，只写 last_error
    stubSourceList([
      makeSource({ id: 1, last_scan_at: successAt, last_error: "磁盘路径不存在" }),
    ]);
    invokeMock.mockClear();
    await eventHandlers.get("scan-complete")!({ payload: { source_id: 1 } });

    expect(store.sources[0].lastScanned).toBe(new Date(successAt).toLocaleString());
    expect(store.sources[0].lastScanned).not.toBe("刚刚扫描");
    expect(store.sources[0].lastError).toBe("磁盘路径不存在");
    // 曲库列表同样以后端为准重拉，而不是本地猜测
    const commands = invokeMock.mock.calls.map((call) => call[0]);
    expect(commands).toContain("source_list");
    expect(commands).toContain("library_get_tracks");
  });

  /**
   * CR-006 的早退场景：数据库写不进去（连接池耗尽、DB 不可用）时，表里仍是上一次的状态。
   * 事件带 persisted=false，前端必须改用事件里的原因，否则用户只看到"点了没反应"。
   */
  it("终态没落库时直接展示事件带来的原因", async () => {
    const { usePlayerStore } = await import("../stores/player");
    const successAt = "2026-09-18T02:00:00Z";
    // 表里既没有本轮错误，也没有本轮时间——只有上一次的
    stubSourceList([makeSource({ id: 1, last_scan_at: successAt, last_error: "上一次的错误" })]);

    const store = usePlayerStore();
    await store.fetchSources();
    await vi.waitFor(() => {
      expect(eventHandlers.has("scan-complete")).toBe(true);
    });

    await eventHandlers.get("scan-complete")!({
      payload: {
        source_id: 1,
        success: false,
        error_code: "db_unavailable",
        message: "扫描无法开始：本地数据库正忙，请稍后重试",
        persisted: false,
      },
    });

    expect(store.sources[0].lastError).toBe("扫描无法开始：本地数据库正忙，请稍后重试");
  });

  it("已落库的失败沿用数据库状态，不用事件覆盖", async () => {
    const { usePlayerStore } = await import("../stores/player");
    // 表里刻意留一条与事件不同的文案：persisted=true 时以回读结果为准，
    // 这条断言锁住"事件只兜底、不抢戏"，避免两处口径互相覆盖。
    stubSourceList([makeSource({ id: 1, last_error: "表里的文案" })]);

    const store = usePlayerStore();
    await store.fetchSources();
    await vi.waitFor(() => {
      expect(eventHandlers.has("scan-complete")).toBe(true);
    });

    await eventHandlers.get("scan-complete")!({
      payload: {
        source_id: 1,
        success: false,
        error_code: "credential_unresolved",
        message: "事件里的文案",
        persisted: true,
      },
    });

    expect(store.sources[0].lastError).toBe("表里的文案");
  });

  it("成功终态不会凭空造出错误", async () => {
    const { usePlayerStore } = await import("../stores/player");
    stubSourceList([makeSource({ id: 1, last_scan_at: "2026-09-18T04:00:00Z" })]);

    const store = usePlayerStore();
    await store.fetchSources();
    await vi.waitFor(() => {
      expect(eventHandlers.has("scan-complete")).toBe(true);
    });

    await eventHandlers.get("scan-complete")!({
      payload: { source_id: 1, success: true, error_code: null, message: null, persisted: true },
    });

    expect(store.sources[0].lastError).toBeUndefined();
  });

  it("scan-progress 只在来源存在时更新进度文本", async () => {
    const { usePlayerStore } = await import("../stores/player");
    stubSourceList([makeSource({ id: 1 })]);
    const store = usePlayerStore();
    await store.fetchSources();
    await vi.waitFor(() => {
      expect(eventHandlers.has("scan-progress")).toBe(true);
    });

    eventHandlers.get("scan-progress")!({
      payload: { source_id: 1, scanned_count: 7, skipped_count: 2, current_path: "a.mp3" },
    });
    expect(store.sources[0].lastScanned).toBe("扫描中: 7 首，跳过 2...");

    // 未知 source_id 不应抛错（并发删除来源时事件仍可能到达）
    expect(() =>
      eventHandlers.get("scan-progress")!({
        payload: { source_id: 999, scanned_count: 1, current_path: "b.mp3" },
      }),
    ).not.toThrow();
  });
});

describe("playerStore.scanSource", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
    eventHandlers.clear();
  });

  it("命令失败时标记为 Error，成功时把结论留给事件", async () => {
    const { usePlayerStore } = await import("../stores/player");
    stubSourceList([makeSource({ id: 1 }), makeSource({ id: 2 })]);
    const store = usePlayerStore();
    await store.fetchSources();

    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "source_scan") {
        if (args?.sourceId === 2) throw new Error("来源已禁用");
        return undefined;
      }
      return [];
    });

    await store.scanSource(2);
    expect(store.sources[1].lastScanned).toBe("Error");

    await store.scanSource(1);
    // source_scan 立即返回，真正的进度/结果由 scan-progress / scan-complete 推送
    expect(store.sources[0].lastScanned).toBe("Scanning...");
    expect(store.sources[0].lastError).toBeUndefined();
  });

  it("未知 id 不做任何状态改写", async () => {
    const { usePlayerStore } = await import("../stores/player");
    stubSourceList([makeSource({ id: 1 })]);
    const store = usePlayerStore();
    await store.fetchSources();
    invokeMock.mockClear();

    await store.scanSource(42);
    expect(invokeMock).not.toHaveBeenCalled();
    expect(store.sources[0].lastScanned).toBe("Never");
  });
});
