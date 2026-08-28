/**
 * AI 能力模块统一出口：
 * - embed / chat：按设置自动路由（本地小模型优先，云端 API 可选）
 * - resolveProvider / currentEngine / syncCloudConfig：引擎管理与配置同步
 * - localChatModelStatus / downloadLocalChatModel / deleteLocalChatModel / localChatModelSize：本地生成式模型管理
 * - localEmbedModelStatus / downloadLocalEmbedModel / deleteLocalEmbedModel / localEmbedModelSize：本地嵌入模型管理
 * - LocalProvider / CloudProvider：两个引擎实现
 * - cosine / chunkText / semanticDiff：语义对比工具函数
 */
export {
  embed,
  chat,
  chatWithTools,
  resolveProvider,
  currentEngine,
  syncCloudConfig,
  syncLocalServerConfig,
  syncLocalChatModel,
  localEngineStatus,
  localChatModelStatus,
  downloadLocalChatModel,
  deleteLocalChatModel,
  localChatModelSize,
  localEmbedModelStatus,
  downloadLocalEmbedModel,
  deleteLocalEmbedModel,
  localEmbedModelSize,
} from "./router";
export { LocalProvider, cosine, chunkText, semanticDiff, formatBytes, SIM_SAME, SIM_REWRITE } from "./local";
export { CloudProvider, cloudDiag, formatDiag } from "./cloud";
export { CLOUD_AI_PRESETS } from "./presets";
export type { CloudAiPreset } from "./presets";
export type { AiProvider, ChatMessage, AiMode, TextChunk, SemanticDiffEntry, ToolCall, ChatReply, ToolDefinition } from "./types";
export type { ChatModelProgress, ChatModelState } from "./local";