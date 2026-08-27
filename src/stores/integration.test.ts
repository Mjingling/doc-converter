import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";

/**
 * 前端集成测试：验证多个 Store 协作和跨模块数据流
 */

// mock API 层
vi.mock("../api", () => ({
  getEngineStatus: vi.fn(),
}));

import { getEngineStatus } from "../api";
import { useEngineStore } from "./engine";
import { useSettingsStore } from "./settings";
import { useHistoryStore } from "./history";

const mockGetEngineStatus = vi.mocked(getEngineStatus);

describe("Store 集成测试", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it("settings + engine store 可同时使用且状态独立", () => {
    const settings = useSettingsStore();
    const engine = useEngineStore();
    // 修改 settings 不影响 engine
    settings.setLocale("en-US");
    expect(engine.mode).toBe("builtin");
    // 修改 engine 不影响 settings
    engine.available = true;
    engine.useLibreOffice();
    expect(settings.locale).toBe("en-US");
    expect(engine.mode).toBe("libreoffice");
  });

  it("engine refresh 后 mode 变化不影响 settings", async () => {
    const settings = useSettingsStore();
    const engine = useEngineStore();
    engine.mode = "libreoffice";
    mockGetEngineStatus.mockResolvedValue({ available: false, path: null });
    await engine.refresh();
    // engine 回退到 builtin
    expect(engine.mode).toBe("builtin");
    // settings 不受影响
    expect(settings.defaultOutDir).toBe("");
  });

  it("settings ai.mode 变更后 engine 状态不受影响", () => {
    const settings = useSettingsStore();
    const engine = useEngineStore();
    settings.setAiConfig({
      ...settings.ai,
      mode: "cloud",
    });
    // engine store 的 mode 是引擎模式（builtin/libreoffice），不是 AI mode
    expect(engine.mode).toBe("builtin");
    expect(settings.ai.mode).toBe("cloud");
  });

  it("history store 独立于其他 store", async () => {
    const history = useHistoryStore();
    const settings = useSettingsStore();
    await history.add({ kind: "merge", name: "test.pdf", inputs: [], outputs: [], ok: true });
    expect(history.items.length).toBe(1);
    // 修改 settings 不影响 history
    settings.setTheme("dark");
    expect(history.items.length).toBe(1);
  });

  it("多个 store 状态持久化互不干扰", () => {
    const settings = useSettingsStore();
    const engine = useEngineStore();
    const history = useHistoryStore();
    // 分别修改
    settings.setDefaultOutDir("/output");
    engine.available = true;
    // 验证各自状态
    expect(settings.defaultOutDir).toBe("/output");
    expect(engine.available).toBe(true);
    expect(history.items).toEqual([]);
  });
});
