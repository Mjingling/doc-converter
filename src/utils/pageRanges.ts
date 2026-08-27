/**
 * 页码范围解析：把 "1-3,5,8-9" 解析为 1 基页码数组
 * （PDF 渲染面板「指定页码」输入框用）
 */

/** 解析失败时抛出的错误（携带 i18n key 供面板提示） */
export class PageRangeError extends Error {}

export function parsePageRanges(input: string, totalPages: number): number[] {
  const result: number[] = [];
  const seen = new Set<number>();
  const parts = input.split(",").map((s) => s.trim()).filter((s) => s.length > 0);
  if (parts.length === 0) throw new PageRangeError("empty");
  for (const part of parts) {
    const m = part.match(/^(\d+)\s*-\s*(\d+)$/);
    if (m) {
      const start = Number(m[1]);
      const end = Number(m[2]);
      if (start < 1 || end < start) throw new PageRangeError("invalid");
      if (end > totalPages) throw new PageRangeError("outOfRange");
      for (let p = start; p <= end; p++) {
        if (!seen.has(p)) {
          seen.add(p);
          result.push(p);
        }
      }
      continue;
    }
    if (/^\d+$/.test(part)) {
      const p = Number(part);
      if (p < 1) throw new PageRangeError("invalid");
      if (p > totalPages) throw new PageRangeError("outOfRange");
      if (!seen.has(p)) {
        seen.add(p);
        result.push(p);
      }
      continue;
    }
    throw new PageRangeError("invalid");
  }
  return result;
}
