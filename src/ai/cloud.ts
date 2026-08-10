import { invoke } from "@tauri-apps/api/core";
import type { AiProvider, ChatMessage } from "./types";
import type { CloudAiConfig } from "../stores/settings";

/**
 * 云端 AI 提供方：请求经 Rust 后端转发到 OpenAI 兼容 API
 * （避免 WebView 直连的 CORS/ATS 限制，密钥不暴露在前端环境）
 */
export class CloudProvider implements AiProvider {
  readonly name = "cloud";
  private config: CloudAiConfig;

  constructor(config: CloudAiConfig) {
    this.config = config;
  }

  /** 更新配置（设置变更后调用，provider 为单例复用） */
  updateConfig(config: CloudAiConfig) {
    this.config = config;
  }

  async status(): Promise<"ready" | "loading" | "unavailable"> {
    if (this.config.baseUrl && this.config.apiKey) return "ready";
    return "unavailable";
  }

  async embed(texts: string[]): Promise<number[][]> {
    if (texts.length === 0) return [];
    return invoke<number[][]>("ai_cloud_embed", {
      texts,
      model: this.config.embeddingModel,
      baseUrl: this.config.baseUrl,
      apiKey: this.config.apiKey,
    });
  }

  async chat(messages: ChatMessage[]): Promise<string> {
    return invoke<string>("ai_cloud_chat", {
      messages,
      model: this.config.chatModel,
      baseUrl: this.config.baseUrl,
      apiKey: this.config.apiKey,
    });
  }
}