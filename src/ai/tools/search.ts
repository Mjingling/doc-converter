/**
 * 网页搜索工具：让 AI 助手能查询实时信息（新闻 / 天气 / 最新版本等）
 *
 * 提供商由设置驱动（设置 → AI 能力 → 网页搜索）：
 * - zhipu：复用云端 API 的密钥与 baseUrl（智谱 web-search，付费按次）
 * - tavily：独立免费 key（settings.ai.search.tavilyKey）
 * 未开启或密钥缺失时工具不出现在模型的能力列表（available 动态过滤）
 */
import { webSearch } from "../../api";
import { useSettingsStore } from "../../stores/settings";
import type { AiTool } from "./types";
import { numArg, strArg } from "./utils";

/** 搜索可用性（供工具注册过滤与系统提示词切换共用） */
export function searchAvailable(): boolean {
  const settings = useSettingsStore();
  const provider = settings.ai.search?.provider;
  if (provider === "zhipu") {
    return !!settings.ai.cloud.baseUrl && !!settings.ai.cloud.apiKey;
  }
  if (provider === "tavily") {
    return !!settings.ai.search.tavilyKey;
  }
  return false;
}

/** 网页搜索 */
const webSearchTool: AiTool = {
  name: "web_search",
  description:
    "搜索互联网并返回结果列表（标题 + 链接 + 摘要）。用于查询实时或可能已变化的信息：" +
    "新闻、天气、价格、软件最新版本、近期事件等。回答此类问题前应先调用本工具，而不是凭记忆作答。",
  available: searchAvailable,
  parameters: {
    type: "object",
    properties: {
      query: { type: "string", description: "搜索关键词（按目标语言习惯组织，如中文问题用中文关键词）" },
      max_results: { type: "number", description: "返回条数 1-10（默认 8）" },
    },
    required: ["query"],
    additionalProperties: false,
  },
  async execute(args) {
    const query = strArg(args, "query");
    if (!query) return { ok: false, message: "参数错误：缺少 query" };
    const maxResults = numArg(args, "max_results", 8);

    const settings = useSettingsStore();
    const provider = settings.ai.search.provider;
    if (!searchAvailable()) {
      return {
        ok: false,
        message: "网页搜索未开启或密钥未配置：请让用户到「设置 → AI 能力 → 网页搜索」开启",
      };
    }
    // 智谱复用云端配置；Tavily 用独立 key（baseUrl 传空，Rust 侧固定端点）
    const apiKey = provider === "tavily" ? settings.ai.search.tavilyKey : settings.ai.cloud.apiKey;
    const baseUrl = provider === "tavily" ? "" : settings.ai.cloud.baseUrl;

    const results = await webSearch(provider, apiKey, baseUrl, query, maxResults);
    if (!results.length) {
      return { ok: true, message: `搜索「${query}」无结果，建议换个关键词` };
    }
    const lines = results.map(
      (r, i) => `${i + 1}. ${r.title}\n   ${r.link}\n   ${r.snippet || "（无摘要）"}`,
    );
    return {
      ok: true,
      message: `搜索「${query}」共 ${results.length} 条结果：\n\n${lines.join("\n\n")}\n\n请基于以上结果回答，并注明信息来源链接。`,
    };
  },
};

/** 搜索工具注册表 */
export const searchTools: AiTool[] = [webSearchTool];
