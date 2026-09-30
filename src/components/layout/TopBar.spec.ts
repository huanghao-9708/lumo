import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mount, flushPromises } from "@vue/test-utils";

/**
 * DM-04 极简交互：TopBar「体验模式」菜单（desktop-modes 02 §5 DM-04 验收 1/5）。
 *
 * 覆盖：菜单打开/关闭（含 Esc）、选中态（aria-checked）、选择即切换体验模式
 * 且不改主题；两维独立（切极简不动窗口形态）。
 */

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("../../utils/tauriInvoke", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    close: vi.fn(),
  }),
}));

import TopBar from "./TopBar.vue";
import { useDesktopModeStore } from "../../stores/desktopMode";
import { useUiStore } from "../../stores/ui";

beforeEach(() => {
  setActivePinia(createPinia());
  localStorage.clear();
  invokeMock.mockReset();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === "desktop_get_preferences") {
      return Promise.resolve({
        preferences: {
          schemaVersion: 1, experienceMode: "normal", windowForm: "full",
          fullGeometry: null, miniGeometry: null, miniAlwaysOnTop: false,
        },
        fileExisted: true,
      });
    }
    return Promise.resolve(undefined);
  });
});

describe("TopBar 体验模式菜单", () => {
  it("打开菜单显示两个选项与选中态，选择极简即切换体验模式", async () => {
    const desktopMode = useDesktopModeStore();
    await desktopMode.init();

    const wrapper = mount(TopBar, { attachTo: document.body });
    const trigger = wrapper.find('button[title="体验模式"]');
    expect(trigger.exists()).toBe(true);

    // 打开菜单
    await trigger.trigger("click");
    await flushPromises();
    const menu = wrapper.find('[role="menu"]');
    expect(menu.exists()).toBe(true);
    expect(trigger.attributes("aria-expanded")).toBe("true");

    const items = wrapper.findAll('[role="menuitemradio"]');
    expect(items).toHaveLength(2);
    expect(items[0]!.attributes("aria-checked")).toBe("true");  // normal 选中
    expect(items[1]!.attributes("aria-checked")).toBe("false");

    // 选择极简 → 体验切换、菜单关闭、形态不变（两维独立）
    await items[1]!.trigger("click");
    await flushPromises();
    expect(desktopMode.experienceMode).toBe("minimal");
    expect(desktopMode.windowForm).toBe("full");
    expect(wrapper.find('[role="menu"]').exists()).toBe(false);

    // 主题未被模式改写
    const ui = useUiStore();
    ui.setDarkMode(true);
    await desktopMode.setExperienceMode("normal");
    await desktopMode.setExperienceMode("minimal");
    expect(ui.isDarkMode).toBe(true);

    wrapper.unmount();
  });

  it("Esc 关闭菜单（focus 归还触发按钮），再次打开选中态跟随当前体验", async () => {
    const desktopMode = useDesktopModeStore();
    await desktopMode.init();
    await desktopMode.setExperienceMode("minimal");

    const wrapper = mount(TopBar, { attachTo: document.body });
    const trigger = wrapper.find('button[title="体验模式"]');
    await trigger.trigger("click");
    await flushPromises();

    await trigger.trigger("keydown.down"); // 也允许方向键打开
    expect(wrapper.find('[role="menu"]').exists()).toBe(true);

    const menuEl = wrapper.find('[role="menu"]');
    await menuEl.trigger("keydown", { key: "Escape" });
    expect(wrapper.find('[role="menu"]').exists()).toBe(false);
    expect(desktopMode.experienceMode).toBe("minimal");

    // 重新打开：极简处于选中态
    await trigger.trigger("click");
    await flushPromises();
    const items = wrapper.findAll('[role="menuitemradio"]');
    expect(items[1]!.attributes("aria-checked")).toBe("true");

    wrapper.unmount();
  });
});
