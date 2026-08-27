/**
 * AI 翻译纯函数：分块 / 语言码 / 围栏剥离（与 Tauri / AI 引擎解耦，可独立单测）
 */

/** 每块目标字符数（超过则按段落/句子切分） */
const CHUNK_CHAR_LIMIT = 2500;
/** 分块数上限（约 20 万字符），防止失控的 token 成本 */
const MAX_CHUNKS = 80;
/** 单块翻译失败重试次数 */
const MAX_ATTEMPTS = 3;

/** 按段落边界分块：空行分段贪心累计；超长段落（>2×limit）先按句末标点切，再硬切 */
export function chunkText(text: string, limit = CHUNK_CHAR_LIMIT): string[] {
  const paras = text.split(/\n{2,}/).flatMap((p) =>
    p.length > limit * 2 ? splitLongParagraph(p, limit) : [p],
  );
  const chunks: string[] = [];
  let cur = "";
  for (const p of paras) {
    // 段落本身超限（但未到 2×）：单独成块
    if (p.length > limit) {
      if (cur) {
        chunks.push(cur);
        cur = "";
      }
      chunks.push(p);
      continue;
    }
    if (cur && cur.length + p.length + 2 > limit) {
      chunks.push(cur);
      cur = p;
    } else {
      cur = cur ? `${cur}\n\n${p}` : p;
    }
  }
  if (cur) chunks.push(cur);
  return chunks;
}

/** 超长段落切分：先按句末标点（。！？!?…；;）聚合，单句仍超长则按 limit 硬切 */
function splitLongParagraph(p: string, limit: number): string[] {
  const sentences = p
    .split(/(?<=[。！？!?…；;])/)
    .flatMap((s) => (s.length > limit ? hardSplit(s, limit) : [s]));
  const parts: string[] = [];
  let cur = "";
  for (const s of sentences) {
    if (cur && cur.length + s.length > limit) {
      parts.push(cur);
      cur = s;
    } else {
      cur += s;
    }
  }
  if (cur) parts.push(cur);
  return parts;
}

/** 硬切：按字符数切，尽量在 UTF-16 边界（代理对不断开） */
function hardSplit(s: string, limit: number): string[] {
  const parts: string[] = [];
  let start = 0;
  while (start < s.length) {
    let end = Math.min(start + limit, s.length);
    // 回退到非代理对尾部
    if (end < s.length) {
      const code = s.charCodeAt(end - 1);
      if (code >= 0xd800 && code <= 0xdbff) end -= 1;
    }
    parts.push(s.slice(start, end));
    start = end;
  }
  return parts;
}

/** 常见语言 → 文件名安全缩写；未知语言则 sanitize 原文 */
export function langCode(targetLang: string): string {
  const map: Record<string, string> = {
    中文: "zh", 汉语: "zh", 简体中文: "zh", 繁体中文: "zh-tw", chines: "zh",
    英语: "en", 英文: "en", english: "en",
    日语: "ja", 日文: "ja", japanese: "ja",
    韩语: "ko", 韩文: "ko", korean: "ko",
    法语: "fr", 法文: "fr", french: "fr",
    德语: "de", 德文: "de", german: "de",
    西班牙语: "es", spanish: "es",
    俄语: "ru", 俄文: "ru", russian: "ru",
    葡萄牙语: "pt", portuguese: "pt",
    意大利语: "it", italian: "it",
    阿拉伯语: "ar", arabic: "ar",
    泰语: "th", thai: "th",
    越南语: "vi", vietnamese: "vi",
  };
  const key = targetLang.trim().toLowerCase();
  if (map[key]) return map[key];
  // ISO 码本身（2-3 位字母）直接用
  if (/^[a-z]{2,3}(-[a-z]{2})?$/i.test(key)) return key;
  // 其他语言名：仅保留字母数字，截短
  const sanitized = key.replace(/[^\p{L}\p{N}]+/gu, "").slice(0, 8);
  return sanitized || "translated";
}

/** 去掉模型可能包裹的 ``` 围栏（整块首尾同时包才去除） */
export function stripFences(s: string): string {
  let t = s.trim();
  const fence = /^```[^\n]*\n([\s\S]*?)\n?```$/;
  let m = t.match(fence);
  while (m) {
    t = m[1].trim();
    m = t.match(fence);
  }
  return t;
}

