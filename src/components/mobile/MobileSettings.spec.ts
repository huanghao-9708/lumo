import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import MobileSettings from "./MobileSettings.vue";
import { usePlayerStore, type MusicSource } from "../../stores/player";

/**
 * 移动端设置页的来源失败提示（I3/G-08 的移动端分支）。
 *
 * 桌面端有独立弹窗，移动端直接列在来源行下方。两端共用同一份
 * playerStore.lastError，这里只锁"移动端确实渲染了它"，
 * 避免修了桌面漏了移动。
 */

vi.mock("../../utils/tauriInvoke", () => ({
  invoke: vi.fn(async (cmd: string) => {
    switch (cmd) {
      case "app_get_version":
        return "1.8.1";
      case "sync_get_config":
        return { enabled: false };
      case "storage_get_db_size":
      case "library_get_cache_size":
      case "playback_get_audio_cache_size":
        return 0;
      default:
        return [];
    }
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

function mountWith(sources: MusicSource[]) {
  const store = usePlayerStore();
  store.sources = sources;
  return mount(MobileSettings);
}

describe("MobileSettings 来源扫描状态", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("渲染 last_error，移动端用户能看到扫描失败原因", async () => {
    const wrapper = mountWith([
      {
        id: 1,
        kind: "webdav",
        name: "NAS 音乐库",
        path: "/music",
        isEnabled: true,
        lastScanned: "2026/9/17 09:02:03",
        lastError: "WebDAV 服务器返回 503",
      },
    ]);
    await wrapper.vm.$nextTick();

    const errors = wrapper.findAll(".text-status-error");
    expect(errors.length).toBeGreaterThan(0);
    expect(errors.map((node) => node.text())).toContain("WebDAV 服务器返回 503");
    expect(wrapper.text()).toContain("NAS 音乐库");
  });

  it("没有 last_error 时不出现错误行", () => {
    const wrapper = mountWith([
      {
        id: 2,
        kind: "local",
        name: "本地音乐",
        path: "D:/Music",
        isEnabled: true,
        lastScanned: "Never",
      },
    ]);

    expect(wrapper.findAll(".text-status-error")).toHaveLength(0);
    expect(wrapper.text()).toContain("本地");
  });
});
