import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

/**
 * SideNav 组件冒烟测试：验证渲染和基本交互
 */

// mock vue-i18n
vi.mock("vue-i18n", () => ({
  useI18n: () => ({
    t: (key: string) => key, // 直接返回 key 作为翻译
  }),
}));

// mock naive-ui
vi.mock("naive-ui", () => ({
  NIcon: { name: "NIcon", template: "<span class='n-icon'><slot/></span>" },
  NDropdown: { name: "NDropdown", template: "<div class='n-dropdown'><slot/></div>" },
  useMessage: () => ({
    success: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
    error: vi.fn(),
  }),
}));

// mock @tauri-apps/plugin-opener
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

// mock @vicons/ionicons5 - 返回简单的组件占位
vi.mock("@vicons/ionicons5", () => {
  const icons = [
    "GitMergeOutline", "GitBranchOutline", "ArchiveOutline",
    "DocumentTextOutline", "ImageOutline", "DocumentOutline",
    "GridOutline", "EaselOutline", "SwapHorizontalOutline",
    "HeartOutline", "WaterOutline", "RefreshOutline", "LockClosedOutline",
    "ImagesOutline", "TimeOutline", "CopyOutline", "CutOutline",
    "InformationCircleOutline", "ResizeOutline", "BookmarkOutline",
    "DocumentAttachOutline", "ContractOutline", "ColorPaletteOutline", "SchoolOutline", "LanguageOutline", "CreateOutline",
    "GlobeOutline", "TextOutline", "SparklesOutline", "ChatbubblesOutline",
    "SearchOutline", "CloseOutline", "SettingsOutline", "HappyOutline",
  ];
  const result: Record<string, any> = {};
  for (const name of icons) {
    result[name] = { name, template: `<i data-icon="${name}"></i>` };
  }
  return result;
});

// mock 子组件
vi.mock("../components/DonateModal.vue", () => ({
  default: { name: "DonateModal", template: "<div class='donate-modal'></div>" },
}));

// mock settings store（左下角设置菜单依赖）
vi.mock("../stores/settings", () => ({
  useSettingsStore: vi.fn(() => ({
    locale: "zh-CN",
    theme: "system",
    pet: { enabled: false },
    setLocale: vi.fn(),
    setTheme: vi.fn(),
    setPetEnabled: vi.fn().mockResolvedValue(undefined),
  })),
}));

// mock engine store
vi.mock("../stores/engine", () => ({
  useEngineStore: vi.fn(() => ({
    mode: "builtin",
    available: true,
    useLibreOffice: vi.fn().mockReturnValue(true),
    useBuiltin: vi.fn(),
    refresh: vi.fn().mockResolvedValue(true),
  })),
}));

import SideNav from "../components/SideNav.vue";

describe("SideNav 组件冒烟测试", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  function mountSideNav(props = {}) {
    return mount(SideNav, {
      props: { active: "merge", ...props },
      global: {
        plugins: [createPinia()],
      },
    });
  }

  it("渲染不报错", () => {
    const wrapper = mountSideNav();
    expect(wrapper.exists()).toBe(true);
  });

  it("包含搜索框", () => {
    const wrapper = mountSideNav();
    const searchInput = wrapper.find(".search-input");
    expect(searchInput.exists()).toBe(true);
  });

  it("渲染导航分组", () => {
    const wrapper = mountSideNav();
    const groups = wrapper.findAll(".nav-group");
    expect(groups.length).toBeGreaterThan(0);
  });

  it("渲染所有功能导航项（至少 20 个）", () => {
    const wrapper = mountSideNav();
    const items = wrapper.findAll(".nav-item");
    expect(items.length).toBeGreaterThanOrEqual(20);
  });

  it("active 项有高亮样式", () => {
    const wrapper = mountSideNav({ active: "merge" });
    const activeItem = wrapper.find(".nav-item.active");
    expect(activeItem.exists()).toBe(true);
  });

  it("搜索过滤功能", async () => {
    const wrapper = mountSideNav();
    const searchInput = wrapper.find(".search-input");
    await searchInput.setValue("merge");
    // 搜索后应只剩匹配的项
    const items = wrapper.findAll(".nav-item");
    expect(items.length).toBeLessThan(20);
    expect(items.length).toBeGreaterThan(0);
  });

  it("引擎卡片显示", () => {
    const wrapper = mountSideNav();
    const engineCard = wrapper.find(".engine-card");
    expect(engineCard.exists()).toBe(true);
  });

  it("点击导航项触发 select 事件", async () => {
    const wrapper = mountSideNav();
    const items = wrapper.findAll(".nav-item");
    // 点击第二个项（第一个可能是 AI 助手）
    if (items.length > 1) {
      await items[1].trigger("click");
      expect(wrapper.emitted("select")).toBeTruthy();
    }
  });
});
