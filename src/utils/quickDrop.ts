/**
 * 拖拽直达（QuickDropModal）：按文件扩展名解析可用操作
 */
import { extOf } from "./file";

export type QuickDropAction = "convert" | "compress" | "images2pdf" | "aiSummary";

/** 可转换的文档类扩展名（走 getTargetFormats 列目标格式） */
export const QUICK_DROP_DOC_EXTS = [
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "md", "markdown", "rtf", "csv", "html",
];

/** 图片类扩展名（可合成 PDF） */
export const QUICK_DROP_IMAGE_EXTS = ["png", "jpg", "jpeg", "webp", "bmp", "gif"];

/** 可提取文本的扩展名（可走 AI 摘要） */
export const QUICK_DROP_TEXT_EXTS = ["pdf", "doc", "docx", "txt", "md", "markdown"];

/**
 * 解析单个文件可用的快捷操作列表（保持展示顺序：转换 → 压缩 → 合成 PDF → AI 摘要）
 * 未知扩展名返回空数组
 */
export function parseQuickDropActions(path: string): QuickDropAction[] {
  const ext = extOf(path).toLowerCase();
  if (!ext) return [];
  const actions: QuickDropAction[] = [];
  if (QUICK_DROP_DOC_EXTS.includes(ext)) actions.push("convert");
  if (ext === "pdf") actions.push("compress");
  if (QUICK_DROP_IMAGE_EXTS.includes(ext)) actions.push("images2pdf");
  if (QUICK_DROP_TEXT_EXTS.includes(ext)) actions.push("aiSummary");
  return actions;
}
