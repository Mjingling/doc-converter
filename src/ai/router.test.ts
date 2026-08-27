import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";

/**
 * AI 路由器测试：验证 mode → provider 的路由决策和 auto 回退链
 * mock LocalProvider / CloudProvider 以及 settings store
 */

// 使用 vi.hoisted 确保 mock 函数在 vi.mock 提升时已可用
const {
  mockLocalStatus,
  mockLocalChatStatus,
  mockLocalChat,
  mockLocalEmbed,
  mockLocalUpdateChatModelId,
  mockCloudChat,
  mockCloudEmbed,
  mockCloudUpdateConfig,
  mockCloudChatWithTools,
  mockAiConfig,
} = vi.hoisted(() => ({
  mockLocalStatus: vi.fn(),
  mockLocalChatStatus: vi.fn(),
  mockLocalChat: vi.fn(),
  mockLocalEmbed: vi.fn(),
  mockLocalUpdateChatModelId: vi.fn(),
  mockCloudChat: vi.fn(),
  mockCloudEmbed: vi.fn(),
  mockCloudUpdateConfig: vi.fn(),
  mockCloudChatWithTools: vi.fn(),
  mockAiConfig: {
    mode: "auto" as string,
    localServer: { baseUrl: "http://localhost:11434/v1", chatModel: "", embeddingModel: "" },
    cloud: { baseUrl: "", apiKey: "", embeddingModel: "", chatModel: "" },
    localChatModelId: "onnx-community/Qwen2.5-0.5B-Instruct",
  },
}));

vi.mock("./local", () => ({
  LocalProvider: vi.fn().mockImplementation(() => ({
    name: "local",
    status: mockLocalStatus,
    chatStatus: mockLocalChatStatus,
    chat: mockLocalChat,
    embed: mockLocalEmbed,
    updateChatModelId: mockLocalUpdateChatModelId,
    downloadChatModel: vi.fn(),
    deleteChatModel: vi.fn(),
    chatModelSize: vi.fn(),
  })),
}));

vi.mock("./cloud", () => ({
  CloudProvider: vi.fn().mockImplementation(() => ({
    name: "cloud",
    chat: mockCloudChat,
    embed: mockCloudEmbed,
    updateConfig: mockCloudUpdateConfig,
    chatWithTools: mockCloudChatWithTools,
    status: vi.fn().mockResolvedValue("ready"),
  })),
}));

vi.mock("../stores/settings", () => ({
  useSettingsStore: vi.fn(() => ({
    ai: mockAiConfig,
  })),
}));

// 在 mock 之后导入 router
import { resolveProvider, chat, embed } from "./router";

describe("AI Router", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mockAiConfig.mode = "auto";
  });

  /* ---------- resolveProvider 路由选择 ---------- */

  it("mode=cloud 返回 cloud provider", async () => {
    mockAiConfig.mode = "cloud";
    const provider = await resolveProvider();
    expect(provider.name).toBe("cloud");
  });

  it("mode=local 返回 local provider", async () => {
    mockAiConfig.mode = "local";
    const provider = await resolveProvider();
    expect(provider.name).toBe("local");
  });

  it("mode=local-server 返回 cloud provider（走本地服务）", async () => {
    mockAiConfig.mode = "local-server";
    const provider = await resolveProvider();
    expect(provider.name).toBe("cloud");
    // 应调用 syncLocalServerConfig → cloud.updateConfig
    expect(mockCloudUpdateConfig).toHaveBeenCalled();
  });

  it("mode=auto local ready 返回 local", async () => {
    mockAiConfig.mode = "auto";
    mockLocalStatus.mockResolvedValue("ready");
    const provider = await resolveProvider();
    expect(provider.name).toBe("local");
  });

  it("mode=auto local not ready 返回 cloud", async () => {
    mockAiConfig.mode = "auto";
    mockLocalStatus.mockResolvedValue("loading");
    const provider = await resolveProvider();
    expect(provider.name).toBe("cloud");
  });

  /* ---------- chat 路由和回退 ---------- */

  it("chat cloud 模式直接走 cloud", async () => {
    mockAiConfig.mode = "cloud";
    mockCloudChat.mockResolvedValue("cloud response");
    const result = await chat([{ role: "user", content: "hi" }]);
    expect(result).toBe("cloud response");
    expect(mockCloudChat).toHaveBeenCalled();
    expect(mockLocalChat).not.toHaveBeenCalled();
  });

  it("chat local 模式直接走 local", async () => {
    mockAiConfig.mode = "local";
    mockLocalChat.mockResolvedValue("local response");
    const result = await chat([{ role: "user", content: "hi" }]);
    expect(result).toBe("local response");
    expect(mockLocalChat).toHaveBeenCalled();
    expect(mockCloudChat).not.toHaveBeenCalled();
  });

  it("chat auto 模式 local ready 走 local", async () => {
    mockAiConfig.mode = "auto";
    mockLocalChatStatus.mockResolvedValue("ready");
    mockLocalChat.mockResolvedValue("local response");
    const result = await chat([{ role: "user", content: "hi" }]);
    expect(result).toBe("local response");
    expect(mockLocalChat).toHaveBeenCalled();
  });

  it("chat auto 模式 local 失败回退 cloud 成功", async () => {
    mockAiConfig.mode = "auto";
    mockLocalChatStatus.mockResolvedValue("ready");
    mockLocalChat.mockRejectedValue(new Error("local failed"));
    mockCloudChat.mockResolvedValue("cloud fallback");
    const result = await chat([{ role: "user", content: "hi" }]);
    expect(result).toBe("cloud fallback");
    expect(mockLocalChat).toHaveBeenCalled();
    expect(mockCloudChat).toHaveBeenCalled();
  });

  it("chat auto 模式 local+cloud 都失败抛云端错误", async () => {
    mockAiConfig.mode = "auto";
    mockLocalChatStatus.mockResolvedValue("ready");
    mockLocalChat.mockRejectedValue(new Error("local failed"));
    mockCloudChat.mockRejectedValue(new Error("cloud failed"));
    await expect(chat([{ role: "user", content: "hi" }])).rejects.toThrow("cloud failed");
  });

  it("chat auto 模式 local not ready 走 cloud", async () => {
    mockAiConfig.mode = "auto";
    mockLocalChatStatus.mockResolvedValue("unavailable");
    mockCloudChat.mockResolvedValue("cloud response");
    const result = await chat([{ role: "user", content: "hi" }]);
    expect(result).toBe("cloud response");
    expect(mockCloudChat).toHaveBeenCalled();
    expect(mockLocalChat).not.toHaveBeenCalled();
  });

  /* ---------- embed 路由 ---------- */

  it("embed 路由到正确 provider", async () => {
    mockAiConfig.mode = "cloud";
    mockCloudEmbed.mockResolvedValue([[0.1, 0.2]]);
    const result = await embed(["test"]);
    expect(result).toEqual([[0.1, 0.2]]);
    expect(mockCloudEmbed).toHaveBeenCalledWith(["test"]);
  });
});
