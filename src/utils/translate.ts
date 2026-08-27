/**
 * AI 翻译辅助：按段落聚合为 ~1200 字符的翻译块
 * （逐块送模型翻译，兼顾上下文连贯与单次请求体积）
 */
export const TRANSLATE_CHUNK_LEN = 1200;

export function splitForTranslate(text: string, maxLen = TRANSLATE_CHUNK_LEN): string[] {
  const paras = text
    .split(/\n+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
  const chunks: string[] = [];
  let cur = "";
  for (const p of paras) {
    if (p.length > maxLen) {
      // 超长段落：先落盘当前缓冲，再硬切
      if (cur) {
        chunks.push(cur);
        cur = "";
      }
      for (let i = 0; i < p.length; i += maxLen) {
        chunks.push(p.slice(i, i + maxLen));
      }
      continue;
    }
    if (cur && cur.length + p.length + 1 > maxLen) {
      chunks.push(cur);
      cur = p;
    } else {
      cur = cur ? `${cur}\n${p}` : p;
    }
  }
  if (cur) chunks.push(cur);
  return chunks;
}
