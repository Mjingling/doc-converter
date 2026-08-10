import type { AiMode } from "../stores/settings";

/** AI 引擎模式：local 本地小模型（WebView 内 WASM 推理） / cloud 云端 API（OpenAI 兼容） */
export type { AiMode };

/** 聊天消息（OpenAI 兼容格式） */
export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

/** AI 能力提供方统一接口：本地（Transformers.js）与云端（API 转发）实现同一抽象 */
export interface AiProvider {
  /** 提供方名称：local / cloud */
  readonly name: string;
  /** 引擎就绪状态：ready 可用 / loading 初始化中 / unavailable 不可用（未下载、未配置） */
  status(): Promise<"ready" | "loading" | "unavailable">;
  /** 文本 → 向量（embedding，归一化后返回） */
  embed(texts: string[]): Promise<number[][]>;
  /** 对话补全（生成式任务） */
  chat(messages: ChatMessage[]): Promise<string>;
}

/** 文本分块（语义对比单位） */
export interface TextChunk {
  /** 在原文中的序号（0-based） */
  index: number;
  text: string;
}

/** 语义对比结果：单条块对比 */
export interface SemanticDiffEntry {
  /** same 相同 / rewritten 改写 / added 新增 / removed 删除 */
  status: "same" | "rewritten" | "added" | "removed";
  /** 块文本（added/removed 来自对应侧，same/rewritten 来自 B 侧） */
  text: string;
  /** 与另一侧最相似块的余弦相似度（0~1），added/removed 时无意义 */
  score: number;
}
