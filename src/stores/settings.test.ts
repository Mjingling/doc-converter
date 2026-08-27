import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useSettingsStore } from "./settings";
import type { AiConfig, WatcherConfig } from "./settings";

/**
 * settings store 状态逻辑测试
 * 不 hydrate 时 fileStore 为 null，save() 中 fileStore?.set 静默跳过
 */

// mock plugin-store
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

// mock invoke (get_default_output_dir)
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue("/platform/default"),
}));
describe("useSettingsStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    localStorage.clear();
  });

  /* ---------- 默认值 ---------- */

  it("默认状态所有字段正确", () => {
    const store = useSettingsStore();
    expect(store.defaultOutDir).toBe("");
    expect(store.locale).toBe("system");
    expect(store.theme).toBe("system");
    expect(store.watcher.enabled).toBe(false);
    expect(store.watcher.folder).toBe("");
    expect(store.watcher.targets).toEqual({});
    expect(store.ai.mode).toBe("auto");
    expect(store.ai.localChatModelId).toBe("Qwen/Qwen2.5-0.5B-Instruct");
    expect(store.ai.cloud.baseUrl).toBe("");
    expect(store.ai.cloud.apiKey).toBe("");
    expect(store.ai.cloud.embeddingModel).toBe("text-embedding-3-small");
    expect(store.ai.cloud.chatModel).toBe("gpt-4o-mini");
    expect(store.ai.localServer.baseUrl).toBe("http://localhost:11434/v1");
  });

  /* ---------- setter 方法 ---------- */

  it("setDefaultOutDir 更新目录", () => {
    const store = useSettingsStore();
    store.setDefaultOutDir("/custom/output");
    expect(store.defaultOutDir).toBe("/custom/output");
  });

  it("clearDefaultOutDir 清空目录", () => {
    const store = useSettingsStore();
    store.setDefaultOutDir("/custom/output");
    store.clearDefaultOutDir();
    expect(store.defaultOutDir).toBe("");
  });

  it("setLocale 更新语言", () => {
    const store = useSettingsStore();
    store.setLocale("ja-JP");
    expect(store.locale).toBe("ja-JP");
  });

  it("setTheme 更新主题", () => {
    const store = useSettingsStore();
    store.setTheme("dark");
    expect(store.theme).toBe("dark");
  });

  it("setWatcher 完整对象写入", () => {
    const store = useSettingsStore();
    const watcher: WatcherConfig = {
      enabled: true,
      folder: "/watch/dir",
      targets: { docx: "pdf", xlsx: "csv" },
    };
    store.setWatcher(watcher);
    expect(store.watcher).toEqual(watcher);
  });

  it("setAiConfig cloud 模式", () => {
    const store = useSettingsStore();
    const ai: AiConfig = {
      mode: "cloud",
      localChatModelId: "Qwen/Qwen2.5-0.5B-Instruct",
      search: { provider: "off", tavilyKey: "" },
      localServer: { baseUrl: "http://localhost:11434/v1", chatModel: "", embeddingModel: "" },
      cloud: {
        baseUrl: "https://api.example.com/v1",
        apiKey: "sk-test-key",
        embeddingModel: "text-embedding-3-large",
        chatModel: "gpt-4o",
      },
    };
    store.setAiConfig(ai);
    expect(store.ai.mode).toBe("cloud");
    expect(store.ai.cloud.baseUrl).toBe("https://api.example.com/v1");
    expect(store.ai.cloud.apiKey).toBe("sk-test-key");
  });

  it("setAiConfig local-server 模式", () => {
    const store = useSettingsStore();
    const ai: AiConfig = {
      mode: "local-server",
      localChatModelId: "",
      search: { provider: "off", tavilyKey: "" },
      localServer: { baseUrl: "http://localhost:8080/v1", chatModel: "llama3", embeddingModel: "nomic-embed" },
      cloud: { baseUrl: "", apiKey: "", embeddingModel: "", chatModel: "" },
    };
    store.setAiConfig(ai);
    expect(store.ai.mode).toBe("local-server");
    expect(store.ai.localServer.chatModel).toBe("llama3");
  });

  it("多次 setter 调用状态独立", () => {
    const store = useSettingsStore();
    store.setLocale("en-US");
    store.setTheme("light");
    store.setDefaultOutDir("/out");
    expect(store.locale).toBe("en-US");
    expect(store.theme).toBe("light");
    expect(store.defaultOutDir).toBe("/out");
  });
});

/* ---------- hydrate 测试 ---------- */
describe("useSettingsStore hydrate", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    localStorage.clear();
  });

  it("hydrate 空存储全部默认值", async () => {
    mockStoreHas.mockResolvedValue(false);
    mockStoreGet.mockResolvedValue(undefined);
    const store = useSettingsStore();
    await store.hydrate();
    expect(store.defaultOutDir).toBe("");
    expect(store.locale).toBe("system");
    expect(store.theme).toBe("system");
    expect(store.ai.mode).toBe("auto");
  });

  it("hydrate 正常存储读取所有字段", async () => {
    mockStoreHas.mockResolvedValue(true);
    mockStoreGet.mockResolvedValue({
      defaultOutDir: "/custom",
      locale: "ja-JP",
      theme: "dark",
      watcher: { enabled: true, folder: "/watch", targets: { docx: "pdf" } },
      ai: { mode: "cloud", localChatModelId: "", localServer: { baseUrl: "", chatModel: "", embeddingModel: "" }, cloud: { baseUrl: "https://api.test.com", apiKey: "key", embeddingModel: "emb", chatModel: "chat" } },
    });
    const store = useSettingsStore();
    await store.hydrate();
    expect(store.defaultOutDir).toBe("/custom");
    expect(store.locale).toBe("ja-JP");
    expect(store.theme).toBe("dark");
    expect(store.watcher.enabled).toBe(true);
    expect(store.ai.mode).toBe("cloud");
    expect(store.ai.cloud.baseUrl).toBe("https://api.test.com");
  });

  it("hydrate 部分字段缺失用默认值填充", async () => {
    mockStoreHas.mockResolvedValue(true);
    mockStoreGet.mockResolvedValue({ defaultOutDir: "/partial" }); // 只有 defaultOutDir
    const store = useSettingsStore();
    await store.hydrate();
    expect(store.defaultOutDir).toBe("/partial");
    expect(store.locale).toBe("system"); // 默认值
    expect(store.theme).toBe("system");
    expect(store.ai.mode).toBe("auto");
  });
});
