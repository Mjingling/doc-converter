/**
 * EP07：AI 引擎 —— 本地优先，云端增强
 *
 * 分层：
 * 1. localReply：零依赖规则引擎，离线可用，永不失败
 * 2. cloudChat：任意 OpenAI 兼容接口（用户自带 baseUrl + key）
 * 3. 路由：配置了就优先云端，失败自动降级本地 —— 永远有答案
 */

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface CloudConfig {
  baseUrl: string; // 如 https://open.bigmodel.cn/api/paas/v4
  apiKey: string;
  model: string; // 如 glm-4-flash
}

// ── 本地规则引擎 ─────────────────────────────────────────

const HELP_TEXT = [
  "我是兔小胖（教学版），现在会用规则引擎回复你：",
  "· 说「你好」试试打招呼",
  "· 输入「摘要：+ 一段话」我帮你做个朴素摘要",
  "· 在下方配置好云端接口后，我就能真正聊天啦",
].join("\n");

/** 本地规则回复：纯函数，输入消息文本 → 回复文本 */
export function localReply(text: string): string {
  const t = text.trim();
  if (/^(你好|hi|hello|在吗)/i.test(t)) {
    return "你好呀！我是本地模式的小助手，断网也能陪你～";
  }
  if (/帮助|怎么用|你会什么/.test(t)) {
    return HELP_TEXT;
  }
  if (t.startsWith("摘要：") || t.startsWith("摘要:")) {
    const body = t.slice(3).trim();
    if (!body) return "摘要后面要跟上内容哦，比如「摘要：今天天气不错」";
    // 教学版"摘要"：取前 50 字 + 统计信息，演示流程而非效果
    const head = body.slice(0, 50);
    return `【本地摘要】${head}${body.length > 50 ? "…" : ""}\n（原文 ${body.length} 字，本地处理未上传）`;
  }
  return `（本地模式·回声）你说："${t}"\n\n想让我更聪明？在下方配置一个 OpenAI 兼容接口试试。`;
}

// ── 云端：OpenAI 兼容接口 ────────────────────────────────

export function cloudReady(cfg: CloudConfig): boolean {
  return Boolean(cfg.baseUrl.trim() && cfg.apiKey.trim() && cfg.model.trim());
}

/**
 * 调用 OpenAI 兼容的 /chat/completions。
 * 智谱、DeepSeek、Moonshot、OpenAI、Ollama……都吃这个格式。
 */
export async function cloudChat(cfg: CloudConfig, history: ChatMessage[]): Promise<string> {
  const base = cfg.baseUrl.trim().replace(/\/+$/, "");
  const resp = await fetch(`${base}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${cfg.apiKey.trim()}`,
    },
    body: JSON.stringify({
      model: cfg.model.trim(),
      messages: [
        { role: "system", content: "你是一个乐于助人的桌面办公小助手，回答简洁。" },
        ...history,
      ],
    }),
  });
  if (!resp.ok) {
    throw new Error(`云端请求失败：HTTP ${resp.status}`);
  }
  const data = await resp.json();
  const content = data?.choices?.[0]?.message?.content;
  if (typeof content !== "string") throw new Error("云端返回格式异常");
  return content;
}
