import { invoke } from "@tauri-apps/api/core";
import type { EngineStatus, FormatInfo } from "../types";

/** 获取转换引擎（LibreOffice）状态 */
export function getEngineStatus(): Promise<EngineStatus> {
  return invoke<EngineStatus>("get_engine_status");
}

/** 根据输入文件扩展名获取可转换的目标格式列表（engine: builtin 轻量 / libreoffice 完整） */
export function getTargetFormats(inputPath: string, engine: string): Promise<FormatInfo[]> {
  return invoke<FormatInfo[]>("get_target_formats", { inputPath, engine });
}

/** 执行文档转换（engine: builtin 轻量提取 / libreoffice 完整版式） */
export function convertDocument(
  inputPath: string,
  targetExt: string,
  outDir: string,
  engine: string
): Promise<string> {
  return invoke<string>("convert_document", { inputPath, targetExt, outDir, engine });
}

/** PDF 合并 */
export function pdfMerge(paths: string[], outPath: string): Promise<string> {
  return invoke<string>("pdf_merge", { paths, outPath });
}

/** PDF 拆分（按范围），返回输出文件路径列表 */
export function pdfSplit(
  inputPath: string,
  ranges: [number, number][],
  outDir: string
): Promise<string[]> {
  return invoke<string[]>("pdf_split", { inputPath, ranges, outDir });
}

/** PDF 基础压缩 */
export function pdfCompress(inputPath: string, outPath: string): Promise<string> {
  return invoke<string>("pdf_compress", { inputPath, outPath });
}

/** 获取 PDF 页数 */
export function getPdfPageCount(inputPath: string): Promise<number> {
  return invoke<number>("get_pdf_page_count", { inputPath });
}

/** 按指定页序提取页面（pages 为 1-based 页码数组，支持任意顺序重排） */
export function pdfExtractPages(inputPath: string, outPath: string, pages: number[]): Promise<string> {
  return invoke<string>("pdf_extract_pages", { inputPath, outPath, pages });
}

/** 删除指定范围的页面（ranges 为 [start, end] 列表，1-based） */
export function pdfDeletePages(inputPath: string, outPath: string, ranges: [number, number][]): Promise<string> {
  return invoke<string>("pdf_delete_pages", { inputPath, outPath, ranges });
}

/** PDF 添加平铺文字水印（text 支持中文，opacity 0.05~1.0，color RGB 0~255，fontSize 字号） */
export function pdfWatermark(
  inputPath: string,
  outPath: string,
  text: string,
  opacity: number,
  color: [number, number, number],
  fontSize: number
): Promise<string> {
  return invoke<string>("pdf_watermark", { inputPath, outPath, text, opacity, color, fontSize });
}

/** PDF 添加页码（style: "page" 仅页码 / "pageOf" 页码+总页数） */
export function pdfPageNumbers(inputPath: string, outPath: string, style: string): Promise<string> {
  return invoke<string>("pdf_page_numbers", { inputPath, outPath, style });
}

/** PDF 旋转全部页面（90 / 180 / 270） */
export function pdfRotate(inputPath: string, outPath: string, angle: number): Promise<string> {
  return invoke<string>("pdf_rotate", { inputPath, outPath, angle });
}

/** PDF 加密（打开密码 + 所有者密码，RC4-128） */
export function pdfEncrypt(
  inputPath: string,
  outPath: string,
  userPass: string,
  ownerPass: string
): Promise<string> {
  return invoke<string>("pdf_encrypt", { inputPath, outPath, userPass, ownerPass });
}

/** PDF 解密（移除打开密码） */
export function pdfDecrypt(inputPath: string, outPath: string, password: string): Promise<string> {
  return invoke<string>("pdf_decrypt", { inputPath, outPath, password });
}

/** 多张图片合成一个 PDF（pageSize: "auto" 跟随图片尺寸 / "a4" A4 居中） */
export function imagesToPdf(paths: string[], outPath: string, pageSize: string): Promise<string> {
  return invoke<string>("images_to_pdf", { paths, outPath, pageSize });
}

/** 递归扫描目录，收集指定扩展名的文件（批量转换用，上限 5000 个） */
export function scanDirectory(dir: string, exts: string[]): Promise<string[]> {
  return invoke<string[]>("scan_directory", { dir, exts });
}

/** 打开路径（文件或文件夹） */
export function openPath(path: string): Promise<void> {
  return invoke<void>("open_path", { path });
}

/** 拉取启动参数中的文件路径（Finder「用 DocMorph 打开」首次唤起时使用，取走后清空） */
export function getLaunchFiles(): Promise<string[]> {
  return invoke<string[]>("get_launch_files");
}

/* ---------- 批次 A：轻量引擎扩展 ---------- */

/** 提取 docx 中嵌入的图片到输出目录 */
export function docxExtractImages(inputPath: string, outDir: string): Promise<string[]> {
  return invoke<string[]>("docx_extract_images", { inputPath, outDir });
}

/* ---------- 批次 B：PDF 工具箱扩展 ---------- */

/** 设置 PDF 文档元数据 */
export function pdfMetadata(
  inputPath: string,
  outPath: string,
  title?: string | null,
  author?: string | null,
  subject?: string | null,
  keywords?: string | null,
): Promise<string> {
  return invoke<string>("pdf_metadata", { inputPath, outPath, title, author, subject, keywords });
}

/** 裁剪 PDF 页面 */
export function pdfCrop(
  inputPath: string,
  outPath: string,
  left: number,
  bottom: number,
  right: number,
  top: number,
): Promise<string> {
  return invoke<string>("pdf_crop", { inputPath, outPath, left, bottom, right, top });
}

/** 添加 PDF 书签（大纲） */
export function pdfOutline(
  inputPath: string,
  outPath: string,
  items: [string, number][],
): Promise<string> {
  return invoke<string>("pdf_outline", { inputPath, outPath, items });
}

/** 压缩图片文件（覆盖原文件） */
export function imageCompress(path: string, quality: number): Promise<void> {
  return invoke<void>("image_compress", { path, quality });
}

/* ---------- 批次 C：文件夹监控 ---------- */

/** 监控状态 */
export interface WatcherStatus {
  running: boolean;
  folder: string | null;
}

/** 启动文件夹监控（targets: 扩展名 → 目标扩展名映射，如 docx → pdf） */
export function watcherStart(folder: string, targets: Record<string, string>): Promise<void> {
  return invoke<void>("watcher_start", { folder, targets });
}

/** 停止文件夹监控 */
export function watcherStop(): Promise<void> {
  return invoke<void>("watcher_stop");
}

/** 查询文件夹监控状态 */
export function watcherStatus(): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("watcher_status");
}

/* ---------- 新功能：PDF 提取图片 / 去水印 / 比较 / 网页转 PDF / 批量重命名 ---------- */

/** 提取 PDF 中嵌入的图片到输出目录 */
export function pdfExtractImages(inputPath: string, outDir: string): Promise<string[]> {
  return invoke<string[]>("pdf_extract_images", { inputPath, outDir });
}

/** 移除 PDF 中的水印（Stamp 注解） */
export function pdfRemoveWatermark(inputPath: string, outPath: string): Promise<string> {
  return invoke<string>("pdf_remove_watermark", { inputPath, outPath });
}

/** 比较两个 PDF 的文本差异 */
export interface DiffEntry {
  status: string;
  line: string;
  line_a: number;
  line_b: number;
}

export function pdfCompare(input1: string, input2: string): Promise<DiffEntry[]> {
  return invoke<DiffEntry[]>("pdf_compare", { input1, input2 });
}

/** 提取 PDF 全文文本（语义对比用） */
export function pdfExtractText(inputPath: string): Promise<string> {
  return invoke<string>("pdf_extract_text", { inputPath });
}

/** 提取文档全文文本（AI 摘要用），支持 pdf / docx / txt / md 等 */
export function extractText(inputPath: string): Promise<string> {
  return invoke<string>("extract_text", { inputPath });
}

/** 网页转 PDF */
export function webpageToPdf(url: string, outPath: string): Promise<string> {
  return invoke<string>("webpage_to_pdf", { url, outPath });
}

/** 批量重命名 */
export interface RenameResult {
  old_path: string;
  new_path: string;
  ok: boolean;
  error: string | null;
}

export function batchRename(items: [string, string][]): Promise<RenameResult[]> {
  return invoke<RenameResult[]>("batch_rename", { items });
}

/* ---------- 检查更新 ---------- */

/** 远程版本信息 */
export interface UpdateInfo {
  hasUpdate: boolean;
  latestVersion: string;
  notes: string;
  downloadUrl: string;
}

/** 比较版本号（三段数字，如 0.10.0 > 0.9.0） */
function semverGt(a: string, b: string): boolean {
  const ap = a.split(".").map(Number);
  const bp = b.split(".").map(Number);
  for (let i = 0; i < Math.max(ap.length, bp.length); i++) {
    const diff = (ap[i] ?? 0) - (bp[i] ?? 0);
    if (diff > 0) return true;
    if (diff < 0) return false;
  }
  return false;
}

/** 版本检查 URL（原始版本 JSON 文件，托管在公开的发布小仓库；主代码仓库保持私有） */
const UPDATE_CHECK_URL =
  "https://gitee.com/speed_turbo/doc-converter-release/raw/master/version.json";

/**
 * 在线检查更新
 * @param currentVersion 当前版本号，如 "0.1.0"
 * @returns 有更新时返回 UpdateInfo，网络错误返回 null，已是最新时 hasUpdate=false
 */
export async function checkUpdate(
  currentVersion: string
): Promise<UpdateInfo | null> {
  try {
    // 走 Rust 侧命令拉取：Gitee raw 302 重定向无 CORS 头，WebView fetch 会被拦截
    const raw = await invoke<string>("fetch_update_json", { url: UPDATE_CHECK_URL });
    const data: { version?: string; notes?: string; download_url?: string } =
      JSON.parse(raw);
    if (!data.version || !data.download_url) return null;
    return {
      hasUpdate: semverGt(data.version, currentVersion),
      latestVersion: data.version,
      notes: data.notes ?? "",
      downloadUrl: data.download_url,
    };
  } catch {
    return null;
  }
}
