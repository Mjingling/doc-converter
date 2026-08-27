import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useEngineStore } from "./engine";

/**
 * engine store 业务逻辑测试
 * 不 hydrate 时 fileStore 为 null，save() 中 fileStore?.set 静默跳过
 */

// mock API 层的 getEngineStatus
vi.mock("../api", () => ({
  getEngineStatus: vi.fn(),
}));

// mock plugin-store 的 load 函数
const mockStoreGet = vi.fn();
const mockStoreSet = vi.fn();
const mockStoreHas = vi.fn();

vi.mock("@tauri-apps/plugin-store", () => ({
  load: vi.fn().mockResolvedValue({
    get: (...args: any[]) => mockStoreGet(...args),
    set: (...args: any[]) => mockStoreSet(...args),
    has: (...args: any[]) => mockStoreHas(...args),
  }),
}));

import { getEngineStatus } from "../api";
const mockGetEngineStatus = vi.mocked(getEngineStatus);

describe("useEngineStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    localStorage.clear();
  });

  it("默认状态为 builtin，available=false", () => {
    const store = useEngineStore();
    expect(store.mode).toBe("builtin");
    expect(store.available).toBe(false);
    expect(store.path).toBeNull();
  });

  it("refresh 可用时更新 available/path 并返回 true", async () => {
    const store = useEngineStore();
    mockGetEngineStatus.mockResolvedValue({ available: true, path: "/usr/bin/soffice" });
    const result = await store.refresh();
    expect(result).toBe(true);
    expect(store.available).toBe(true);
    expect(store.path).toBe("/usr/bin/soffice");
  });

  it("refresh libreoffice 模式但不可用时自动回退 builtin", async () => {
    const store = useEngineStore();
    store.mode = "libreoffice";
    mockGetEngineStatus.mockResolvedValue({ available: false, path: null });
    await store.refresh();
    expect(store.mode).toBe("builtin");
    expect(store.available).toBe(false);
  });

  it("refresh builtin 模式不可用时不回退（已是 builtin）", async () => {
    const store = useEngineStore();
    store.mode = "builtin";
    mockGetEngineStatus.mockResolvedValue({ available: false, path: null });
    await store.refresh();
    expect(store.mode).toBe("builtin");
  });

  it("refresh libreoffice 模式且可用时不回退", async () => {
    const store = useEngineStore();
    store.mode = "libreoffice";
    mockGetEngineStatus.mockResolvedValue({ available: true, path: "/opt/lo" });
    await store.refresh();
    expect(store.mode).toBe("libreoffice");
  });

  it("useLibreOffice 不可用返回 false 且不切换", () => {
    const store = useEngineStore();
    store.available = false;
    expect(store.useLibreOffice()).toBe(false);
    expect(store.mode).toBe("builtin");
  });

  it("useLibreOffice 可用时切换 mode 并返回 true", () => {
    const store = useEngineStore();
    store.available = true;
    expect(store.useLibreOffice()).toBe(true);
    expect(store.mode).toBe("libreoffice");
  });

  it("useBuiltin 切回内置引擎", () => {
    const store = useEngineStore();
    store.mode = "libreoffice";
    store.useBuiltin();
    expect(store.mode).toBe("builtin");
  });
});

/* ---------- hydrate 测试 ---------- */
describe("useEngineStore hydrate", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    localStorage.clear();
  });

  it("hydrate 空存储时默认 builtin", async () => {
    mockStoreHas.mockResolvedValue(false);
    mockStoreGet.mockResolvedValue(undefined);
    const store = useEngineStore();
    await store.hydrate();
    expect(store.mode).toBe("builtin");
  });

  it("hydrate 存储有 libreoffice 时读取", async () => {
    mockStoreHas.mockResolvedValue(true);
    mockStoreGet.mockResolvedValue("libreoffice");
    const store = useEngineStore();
    await store.hydrate();
    expect(store.mode).toBe("libreoffice");
  });

  it("hydrate 存储值非法时回退 builtin", async () => {
    mockStoreHas.mockResolvedValue(true);
    mockStoreGet.mockResolvedValue("invalid_mode");
    const store = useEngineStore();
    await store.hydrate();
    expect(store.mode).toBe("builtin");
  });
});
