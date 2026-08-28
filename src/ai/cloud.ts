import { invoke } from "@tauri-apps/api/core";
import type { AiProvider, ChatMessage, ChatReply, ToolDefinition } from "./types";
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
    const reply = await this.chatWithTools(messages, []);
    return reply.content ?? "";
  }

  /** 携带工具定义的对话补全（function calling）；返回 content 与 tool_calls */
  async chatWithTools(messages: ChatMessage[], tools: ToolDefinition[]): Promise<ChatReply> {
    return invoke<ChatReply>("ai_cloud_chat", {
      messages,
      model: this.config.chatModel,
      baseUrl: this.config.baseUrl,
      apiKey: this.config.apiKey,
      tools: tools.length > 0 ? tools : null,
    });
  }
}

/* ---------- 云端连接分阶段诊断（DNS → TCP → HTTP/TLS）：测试连接失败时定位卡点 ---------- */

export interface TcpProbe {
  addr: string;
  ok: boolean;
  ms: number;
  error?: string;
}

export interface CloudDiag {
  dns_addrs: string[];
  dns_ms: number;
  tcp: TcpProbe[];
  http_status?: number;
  http_ms?: number;
  http_error?: string;
}

/** 诊断结果拼成可读文本（纯技术输出，不进 i18n） */
export function formatDiag(d: CloudDiag): string {
  const dns = d.dns_addrs.length ? d.dns_addrs.join(", ") : "无结果";
  const tcp = d.tcp.length
    ? d.tcp.map((p) => `${p.addr} ${p.ok ? p.ms + "ms" : "失败(" + p.error + ")"}`).join("；")
    : "-";
  const http = d.http_status != null ? `${d.http_status} (${d.http_ms}ms)` : d.http_error ?? "未到达";
  return `DNS(${d.dns_ms}ms): ${dns} | TCP: ${tcp} | HTTP: ${http}`;
}

/** 分阶段诊断指定 base_url 的连通性（GET 探测，无需 API key） */
export function cloudDiag(baseUrl: string): Promise<CloudDiag> {
  return invoke<CloudDiag>("ai_cloud_diag", { baseUrl });
}