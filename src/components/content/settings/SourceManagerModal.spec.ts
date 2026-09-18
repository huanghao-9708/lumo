import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import SourceManagerModal from "./SourceManagerModal.vue";
import { usePlayerStore, type MusicSource } from "../../../stores/player";

/**
 * 数据源管理弹窗对"上次扫描结果"的呈现（I3/G-08）。
 *
 * 修复前失败原因只存在数据库里，前端没有任何组件渲染它，用户看到的
 * 永远是"上次成功时间"，把失败说成成功。这里锁死三件事：
 *   1. 有 last_error 时必须渲染错误行；
 *   2. 时间文案要区分"从未扫描成功 / 未扫描 / 上次成功：<时间>"；
 *   3. 扫描进行中不展示上一次的错误，避免误导。
 */

vi.mock("../../../utils/tauriInvoke", () => ({
  invoke: vi.fn(async () => []),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

// 文件夹选择器只在点击"浏览"时用，挂载阶段不应触碰 Tauri
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
}));

function source(overrides: Partial<MusicSource> & { id: number }): MusicSource {
  return {
    kind: "local",
    name: `来源 ${overrides.id}`,
    path: `D:/Music${overrides.id}`,
    isEnabled: true,
    lastScanned: "Never",
    ...overrides,
  };
}

function mountWith(sources: MusicSource[]) {
  const store = usePlayerStore();
  store.sources = sources;
  // AppModal 内容 Teleport 到 body，stub 掉 teleport 才能用 wrapper 查询
  return mount(SourceManagerModal, {
    global: { stubs: { teleport: true } },
  });
}

describe("SourceManagerModal 扫描状态文案", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("从未扫描但有失败记录时显示「从未扫描成功」并渲染原因", () => {
    const wrapper = mountWith([source({ id: 1, lastError: "WebDAV 401 Unauthorized" })]);

    expect(wrapper.text()).toContain("从未扫描成功");
    const errors = wrapper.findAll(".text-status-error");
    expect(errors).toHaveLength(1);
    expect(errors[0].text()).toBe("WebDAV 401 Unauthorized");
  });

  it("从未扫描且没有失败记录时显示「未扫描」，不出现错误行", () => {
    const wrapper = mountWith([source({ id: 2 })]);

    expect(wrapper.text()).toContain("未扫描");
    expect(wrapper.text()).not.toContain("从未扫描成功");
    expect(wrapper.findAll(".text-status-error")).toHaveLength(0);
  });

  it("有 last_error 时时间文案标注「上次成功」，避免把失败时间读成成功时间", () => {
    const lastSuccess = new Date("2026-09-17T01:02:03Z").toLocaleString();
    const wrapper = mountWith([
      source({ id: 3, lastScanned: lastSuccess, lastError: "扫描中途断开" }),
    ]);

    expect(wrapper.text()).toContain(`上次成功：${lastSuccess}`);
    expect(wrapper.findAll(".text-status-error")[0].text()).toBe("扫描中途断开");
  });

  it("上次扫描成功时只显示时间，不加「上次成功」前缀也不报错", () => {
    const lastSuccess = new Date("2026-09-17T01:02:03Z").toLocaleString();
    const wrapper = mountWith([source({ id: 4, lastScanned: lastSuccess })]);

    expect(wrapper.text()).toContain(lastSuccess);
    expect(wrapper.text()).not.toContain("上次成功：");
    expect(wrapper.findAll(".text-status-error")).toHaveLength(0);
  });

  it("扫描进行中不显示时间戳也不重提上次错误", () => {
    const wrapper = mountWith([
      source({ id: 5, lastScanned: "Scanning...", lastError: "上一次的错误" }),
    ]);

    expect(wrapper.text()).not.toContain("上一次的错误");
    expect(wrapper.text()).not.toContain("Scanning...");
    expect(wrapper.findAll(".text-status-error")).toHaveLength(0);
  });

  it("扫描命令本身失败时显示「上次扫描失败」", () => {
    const wrapper = mountWith([source({ id: 6, lastScanned: "Error" })]);

    expect(wrapper.text()).toContain("上次扫描失败");
  });

  it("无来源时渲染空态而不是空白", () => {
    const wrapper = mountWith([]);

    expect(wrapper.text()).toContain("尚未添加数据源");
  });
});
