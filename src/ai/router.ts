import { useSettingsStore } from "../stores/settings";
import { LocalProvider } from "./local";
import { CloudProvider } from "./cloud";
import type { AiProvider, ChatMessage, ChatReply, ToolDefinition } from "./types";
import type { ChatModelProgress, ChatModelState } from "./local";

/**
 * AI 能力路由器：按设置 ai.mode 选择引擎
 * - local：仅本地小模型（WebView 内 WASM）
 * - local-server：连接用户自行部署的本地服务（Ollama / LM Studio 等 OpenAI 兼容端点）
 * - cloud：仅云端 API（OpenAI 兼容，Rust 后端转发）
 * - auto：本地模型就绪用本地，否则回退云端
 */
const local = new LocalProvider();
const cloud = new CloudProvider({
  baseUrl: "",
  apiKey: "",
  embeddingModel: "text-embedding-3-small",
  chatModel: "gpt-4o-mini",
});

/** 同步云端配置到 provider（设置变更后调用） */
export function syncCloudConfig() {
  cloud.updateConfig(useSettingsStore().ai.cloud);
}

/** 同步本地服务配置到 provider（设置变更后调用） */
export function syncLocalServerConfig() {
  const srv = useSettingsStore().ai.localServer;
  cloud.updateConfig({
    baseUrl: srv.baseUrl,
    apiKey: "",
    embeddingModel: srv.embeddingModel,
    chatModel: srv.chatModel,
  });
}

/** 同步本地 chat 模型 ID（设置页变更后调用） */
export function syncLocalChatModel() {
  local.updateChatModelId(useSettingsStore().ai.localChatModelId);
}

/** 携带工具定义的对话补全（AI 助手专用）：强制走云端（本地模型不支持工具调用） */
export async function chatWithTools(messages: ChatMessage[], tools: ToolDefinition[]): Promise<ChatReply> {
  syncCloudConfig();
  return cloud.chatWithTools(messages, tools);
}

/** 解析当前生效的引擎 */
export async function resolveProvider(): Promise<AiProvider> {
  const { ai } = useSettingsStore();
  if (ai.mode === "local-server") {
    syncLocalServerConfig();
    return cloud;
  }
  if (ai.mode === "cloud") return cloud;
  if (ai.mode === "local") return local;
  // auto：本地模型已就绪优先，否则云端
  const s = await local.status();
  return s === "ready" ? local : cloud;
}

/** 当前生效引擎名称（auto 模式下按本地是否就绪实时判断） */
export async function currentEngine(): Promise<"local" | "cloud"> {
  return (await resolveProvider()).name as "local" | "cloud";
}

/** 本地引擎就绪状态（设置页展示用） */
export async function localEngineStatus(): Promise<"ready" | "loading" | "unavailable"> {
  return local.status();
}

/** 本地生成式（chat）模型状态（设置页展示用） */
export async function localChatModelStatus(): Promise<ChatModelState> {
  syncLocalChatModel();
  return local.chatStatus();
}

/** 下载本地 chat 模型（设置页下载按钮；带进度回调） */
export async function downloadLocalChatModel(onProgress: (p: ChatModelProgress) => void): Promise<void> {
  syncLocalChatModel();
  return local.downloadChatModel(onProgress);
}

/** 删除本地 chat 模型缓存，返回删除的文件数 */
export async function deleteLocalChatModel(): Promise<number> {
  syncLocalChatModel();
  return local.deleteChatModel();
}

/** 本地 chat 模型缓存大小（字节） */
export async function localChatModelSize(): Promise<number> {
  syncLocalChatModel();
  return local.chatModelSize();
}

/** 本地 embedding 模型状态（设置页展示用；模型 ID 固定无需同步） */
export async function localEmbedModelStatus(): Promise<ChatModelState> {
  return local.embedStatus();
}

/** 下载本地 embedding 模型（设置页下载按钮；带进度回调） */
export async function downloadLocalEmbedModel(onProgress: (p: ChatModelProgress) => void): Promise<void> {
  return local.downloadEmbedModel(onProgress);
}

/** 删除本地 embedding 模型缓存，返回删除的文件数 */
export async function deleteLocalEmbedModel(): Promise<number> {
  return local.deleteEmbedModel();
}

/** 本地 embedding 模型缓存大小（字节） */
export async function localEmbedModelSize(): Promise<number> {
  return local.embedModelSize();
}

/** 文本 → 向量（按配置自动路由到本地或云端） */
export async function embed(texts: string[]): Promise<number[][]> {
  const provider = await resolveProvider();
  return provider.embed(texts);
}

/** 对话补全（按配置自动路由；local-server 走本地服务，auto 模式本地 chat 模型就绪则用本地，否则走云端） */
export async function chat(messages: ChatMessage[]): Promise<string> {
  const { ai } = useSettingsStore();
  syncLocalChatModel();
  if (ai.mode === "local-server") {
    syncLocalServerConfig();
    return cloud.chat(messages);
  }
  if (ai.mode === "cloud") return cloud.chat(messages);
  if (ai.mode === "local") return local.chat(messages);
  // auto：本地 chat 模型已就绪优先，否则云端
  const s = await local.chatStatus();
  if (s === "ready") {
    try {
      return await local.chat(messages);
    } catch (e) {
      // 本地推理失败（如加载异常）时回退云端；云端也失败则抛云端错误，避免掩盖真实原因
      try {
        return await cloud.chat(messages);
      } catch (cloudErr) {
        throw cloudErr;
      }
    }
  }
  return cloud.chat(messages);
}