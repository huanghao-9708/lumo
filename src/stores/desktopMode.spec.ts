import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

/**
 * DM-01 两维控制器验收测试（desktop-modes 02 §4 DM-01）。
 *
 * 覆盖五条验收：
 *  1. 体验与形态两维独立，四组合均合法；
 *  2. 任意转换不改主题（isDarkMode）；
 *  3. 偏好读取失败/损坏 → 默认 normal+full，不阻塞 loaded；
 *  4. 连续点击只有一次有效转换（phase 串行化 + 过期 token）；
 *  5. 写入失败：内存切换保留、标记未保存、提示一次、不循环重试。
 *
 * 全部 IPC 走 src/utils/tauriInvoke.ts 咽喉点，mock 它即可。
 */

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("../utils/tauriInvoke", () => ({ invoke: invokeMock }));

import { useDesktopModeStore } from "./desktopMode";
import { useUiStore } from "./ui";

const DEFAULT_PREFS = {
  schemaVersion: 1,
  experienceMode: "normal",
  windowForm: "full",
  fullGeometry: null,
  miniGeometry: null,
  miniAlwaysOnTop: false,
};

function okSnapshot(overrides: Record<string, unknown> = {}) {
  return {
    preferences: { ...DEFAULT_PREFS, ...overrides },
    fileExisted: true,
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
  invokeMock.mockReset();
  localStorage.clear();
  // 默认：已有偏好文件且为默认值；各用例按需覆盖
  invokeMock.mockResolvedValue(okSnapshot());
});

describe("DM-01 两维控制器", () => {
  it("验收1：两维独立，四组合均合法，返回完整保留体验模式", async () => {
    const s = useDesktopModeStore();
    await s.init();

    // normal+full → 切极简（体验），形态不变
    expect(await s.setExperienceMode("minimal")).toBe(true);
    expect(s.experienceMode).toBe("minimal");
    expect(s.windowForm).toBe("full");
    expect(s.visualAllowed).toBe(false);

    // 进迷你：体验仍是极简 → minimal+mini
    expect(await s.enterMini()).toBe(true);
    expect(s.windowForm).toBe("mini");
    expect(s.experienceMode).toBe("minimal");

    // 迷你中切体验 → normal+mini
    expect(await s.setExperienceMode("normal")).toBe(true);
    expect(s.experienceMode).toBe("normal");
    expect(s.windowForm).toBe("mini");
    expect(s.visualAllowed).toBe(false);

    // 返回完整 → normal+full（体验保留）
    expect(await s.exitMini()).toBe(true);
    expect(s.windowForm).toBe("full");
    expect(s.experienceMode).toBe("normal");
    expect(s.visualAllowed).toBe(true);

    // 每次转换都提交了一次持久化快照，且提交值与状态一致
    const updates = invokeMock.mock.calls.filter(([cmd]) => cmd === "desktop_update_preferences");
    expect(updates.length).toBe(4);
    const lastPayload = updates[updates.length - 1]![1].preferences;
    expect(lastPayload.experienceMode).toBe("normal");
    expect(lastPayload.windowForm).toBe("full");
  });

  it("验收2：任意转换不改主题", async () => {
    const s = useDesktopModeStore();
    const ui = useUiStore();
    ui.setDarkMode(true);
    await s.init();

    await s.setExperienceMode("minimal");
    await s.enterMini();
    await s.setExperienceMode("normal");
    await s.exitMini();

    expect(ui.isDarkMode).toBe(true);
    expect(ui.followSystem).toBe(false);
  });

  it("验收3：读取失败/损坏回退默认 normal+full，不阻塞启动", async () => {
    invokeMock.mockRejectedValue(new Error("ipc down"));
    const s = useDesktopModeStore();
    await s.init();

    expect(s.loaded).toBe(true);
    expect(s.experienceMode).toBe("normal");
    expect(s.windowForm).toBe("full");
    expect(s.visualAllowed).toBe(true);

    // 默认仍是可操作的：可以正常切换
    expect(await s.enterMini()).toBe(true);
    expect(s.windowForm).toBe("mini");
  });

  it("验收4：转换进行中的连续点击被拒绝，只有一次有效转换", async () => {
    // 第一个 update 挂起模拟慢 IPC；放行后其余 update 立即成功
    let releaseFirstUpdate!: (v: unknown) => void;
    let firstUpdatePending = true;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "desktop_get_preferences") return Promise.resolve(okSnapshot());
      if (cmd === "desktop_update_preferences") {
        if (firstUpdatePending) {
          firstUpdatePending = false;
          return new Promise((resolve) => { releaseFirstUpdate = resolve; });
        }
        return Promise.resolve(undefined);
      }
      return Promise.resolve(null);
    });

    const s = useDesktopModeStore();
    await s.init();

    const first = s.enterMini();
    // 转换未完成（update 未 resolve），第二次转换必须被拒
    expect(s.phase).toBe("entering-mini");
    expect(await s.setExperienceMode("minimal")).toBe(false);
    expect(await s.exitMini()).toBe(false);
    expect(s.experienceMode).toBe("normal"); // 未被第二次点击改写
    expect(s.windowForm).toBe("mini");       // 第一次转换的内存状态已生效

    releaseFirstUpdate(undefined);
    expect(await first).toBe(true);
    expect(s.phase).toBe("idle");
    // 后续转换恢复可用
    expect(await s.setExperienceMode("minimal")).toBe(true);
    expect(s.experienceMode).toBe("minimal");
  });

  it("验收5：写入失败保留内存切换、标记未保存、提示一次且不循环重试", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "desktop_get_preferences") return Promise.resolve(okSnapshot());
      if (cmd === "desktop_update_preferences") return Promise.reject(new Error("disk full"));
      return Promise.resolve(null);
    });

    const s = useDesktopModeStore();
    const ui = useUiStore();
    await s.init();

    expect(await s.enterMini()).toBe(true);
    // 切换保留在内存（不回滚），并标记未保存
    expect(s.windowForm).toBe("mini");
    expect(s.prefsDirty).toBe(true);
    expect(ui.toast?.kind).toBe("error");
    const toastMessage = ui.toast?.message ?? "";
    expect(toastMessage).toContain("未保存");

    // 不循环重试：失败后不产生额外的 update 调用
    const updateCallsAfterFailure = invokeMock.mock.calls.filter(
      ([cmd]) => cmd === "desktop_update_preferences",
    ).length;
    expect(updateCallsAfterFailure).toBe(1);

    // 转换机回到 idle，用户仍可继续操作（本次运行内体验一致）
    expect(s.phase).toBe("idle");
  });

  it("首次运行：旧 lumo_window_size 迁移为完整几何，只执行一次", async () => {
    localStorage.setItem("lumo_window_size", JSON.stringify({ w: 1440, h: 900 }));
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "desktop_get_preferences") {
        return Promise.resolve({ preferences: DEFAULT_PREFS, fileExisted: false });
      }
      return Promise.resolve(undefined);
    });

    const s = useDesktopModeStore();
    await s.init();

    expect(s.fullGeometry).toEqual({ x: 0, y: 0, width: 1440, height: 900, maximized: false });
    const updates = invokeMock.mock.calls.filter(([cmd]) => cmd === "desktop_update_preferences");
    expect(updates.length).toBe(1);
    expect(updates[0]![1].preferences.fullGeometry).toEqual({
      x: 0, y: 0, width: 1440, height: 900, maximized: false,
    });

    // 已有偏好文件时不再迁移
    invokeMock.mockResolvedValue(okSnapshot());
    const s2 = useDesktopModeStore();
    await s2.init();
    expect(s2.fullGeometry).toBeNull();
  });

  it("DM-03：strategyVersion 随有效转换自增，先于异步消费者", async () => {
    const s = useDesktopModeStore();
    await s.init();
    const v0 = s.strategyVersion;

    expect(await s.setExperienceMode("minimal")).toBe(true);
    expect(s.strategyVersion).toBeGreaterThan(v0);
    const v1 = s.strategyVersion;
    expect(await s.enterMini()).toBe(true);
    expect(s.strategyVersion).toBeGreaterThan(v1);
    // visualAllowed 随两维正确派生
    expect(s.visualAllowed).toBe(false);
    expect(await s.exitMini()).toBe(true);
    expect(await s.setExperienceMode("normal")).toBe(true);
    expect(s.visualAllowed).toBe(true);
  });

  it("几何更新：转换期间拒绝提交，避免几何互相覆盖", async () => {
    // 第一个 update 挂起模拟转换进行中；放行后其余 update 立即成功
    let releaseFirstUpdate!: (v: unknown) => void;
    let firstUpdatePending = true;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "desktop_get_preferences") return Promise.resolve(okSnapshot());
      if (cmd === "desktop_update_preferences") {
        if (firstUpdatePending) {
          firstUpdatePending = false;
          return new Promise((resolve) => { releaseFirstUpdate = resolve; });
        }
        return Promise.resolve(undefined);
      }
      return Promise.resolve(null);
    });

    const s = useDesktopModeStore();
    await s.init();

    const pending = s.enterMini();
    const geometryWrite = s.updateFullGeometry({ x: 1, y: 2, width: 800, height: 600, maximized: false });
    expect(await geometryWrite).toBe(false);
    expect(s.fullGeometry).toBeNull();

    releaseFirstUpdate(undefined);
    await pending;
    expect(await s.updateFullGeometry({ x: 1, y: 2, width: 800, height: 600, maximized: true })).toBe(true);
    expect(s.fullGeometry).toEqual({ x: 1, y: 2, width: 800, height: 600, maximized: true });
  });
});
