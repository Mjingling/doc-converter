/**
 * 导航分组数据（SideNav 与命令面板共用）
 * label 为 i18n key，渲染时由调用方 t() 翻译
 */
import type { NavId } from "./types";
import {
  GitMergeOutline, GitBranchOutline, ArchiveOutline, CutOutline,
  WaterOutline, RefreshOutline, LockClosedOutline, ImagesOutline, CopyOutline,
  DocumentTextOutline, ImageOutline, DocumentOutline,
  GridOutline, EaselOutline, SwapHorizontalOutline,
  TimeOutline,
  InformationCircleOutline, ResizeOutline, BookmarkOutline,
  DocumentAttachOutline, ContractOutline, ColorPaletteOutline, SchoolOutline, LanguageOutline, CreateOutline,
  GlobeOutline, TextOutline, SparklesOutline, ChatbubblesOutline,
} from "@vicons/ionicons5";

export interface NavItem {
  id: NavId;
  label: string;
  icon: any;
  color: string;
}

export interface NavGroup {
  title: string;
  engine: "builtin" | "libreoffice" | "none";
  items: NavItem[];
}

/** 导航分组：AI 置顶 → 历史记录 → PDF 处理 → 工具箱 → 文档转换 → 扩展 → 实用工具 */
export const navGroups: NavGroup[] = [
  // AI 助手置顶：默认首页，独立无标题组
  {
    title: "",
    engine: "none",
    items: [
      { id: "aiAssistant", label: "nav.aiAssistant", icon: ChatbubblesOutline, color: "#18a058" },
      { id: "docQa", label: "nav.docQa", icon: SchoolOutline, color: "#18a058" },
      { id: "translate", label: "nav.translate", icon: LanguageOutline, color: "#18a058" },
    ],
  },
  // 历史记录放最前：高频入口，避免被功能项挤到列表底部
  {
    title: "nav.groupHistory",
    engine: "none",
    items: [{ id: "history", label: "nav.history", icon: TimeOutline, color: "#e6494c" }],
  },
  {
    title: "nav.groupPdf",
    engine: "builtin",
    items: [
      { id: "merge", label: "nav.merge", icon: GitMergeOutline, color: "#e6494c" },
      { id: "split", label: "nav.split", icon: GitBranchOutline, color: "#e6494c" },
      { id: "compress", label: "nav.compress", icon: ArchiveOutline, color: "#e6494c" },
      { id: "organize", label: "nav.organize", icon: CutOutline, color: "#e6494c" },
    ],
  },
  {
    title: "nav.groupTools",
    engine: "builtin",
    items: [
      { id: "watermark", label: "nav.watermark", icon: WaterOutline, color: "#e6494c" },
      { id: "rotate", label: "nav.rotate", icon: RefreshOutline, color: "#e6494c" },
      { id: "encrypt", label: "nav.encrypt", icon: LockClosedOutline, color: "#e6494c" },
      { id: "images2pdf", label: "nav.images2pdf", icon: ImagesOutline, color: "#e6494c" },
      { id: "batch", label: "nav.batch", icon: CopyOutline, color: "#e6494c" },
    ],
  },
  {
    title: "nav.groupConvert",
    engine: "libreoffice",
    items: [
      { id: "pdf2word", label: "nav.pdf2word", icon: DocumentTextOutline, color: "#2080f0" },
      { id: "pdf2image", label: "nav.pdf2image", icon: ImageOutline, color: "#2080f0" },
      { id: "pdf2excel", label: "nav.pdf2excel", icon: GridOutline, color: "#2080f0" },
      { id: "word2pdf", label: "nav.word2pdf", icon: DocumentOutline, color: "#2080f0" },
      { id: "excel2pdf", label: "nav.excel2pdf", icon: GridOutline, color: "#2080f0" },
      { id: "ppt2pdf", label: "nav.ppt2pdf", icon: EaselOutline, color: "#2080f0" },
      { id: "convert", label: "nav.convert", icon: SwapHorizontalOutline, color: "#2080f0" },
    ],
  },
  {
    title: "nav.groupExtras",
    engine: "builtin",
    items: [
      { id: "metadata", label: "nav.metadata", icon: InformationCircleOutline, color: "#e6494c" },
      { id: "crop", label: "nav.crop", icon: ResizeOutline, color: "#e6494c" },
      { id: "outline", label: "nav.outline", icon: BookmarkOutline, color: "#e6494c" },
      { id: "pdfExtractImages", label: "nav.pdfExtractImages", icon: ImageOutline, color: "#e6494c" },
      { id: "removeWatermark", label: "nav.removeWatermark", icon: WaterOutline, color: "#e6494c" },
      { id: "comparePdf", label: "nav.comparePdf", icon: DocumentTextOutline, color: "#e6494c" },
      { id: "pdfRender", label: "nav.pdfRender", icon: ImageOutline, color: "#e6494c" },
      { id: "signature", label: "nav.signature", icon: CreateOutline, color: "#e6494c" },
    ],
  },
  {
    title: "nav.groupUtils",
    engine: "none",
    items: [
      { id: "webToPdf", label: "nav.webToPdf", icon: GlobeOutline, color: "#18a058" },
      { id: "docxExtract", label: "nav.docxExtract", icon: DocumentAttachOutline, color: "#18a058" },
      { id: "imageCompress", label: "nav.imageCompress", icon: ContractOutline, color: "#18a058" },
      { id: "imageConvert", label: "nav.imageConvert", icon: ColorPaletteOutline, color: "#18a058" },
      { id: "batchRename", label: "nav.batchRename", icon: TextOutline, color: "#18a058" },
      { id: "aiSummary", label: "nav.aiSummary", icon: SparklesOutline, color: "#18a058" },
    ],
  },
];

/** 拍平所有导航项（保持分组顺序） */
export function flattenNavItems(): NavItem[] {
  return navGroups.flatMap((g) => g.items);
}

/**
 * 模糊匹配：连续子串匹配（忽略大小写）
 * query 为空视为全部匹配
 */
export function fuzzyMatch(query: string, text: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return text.toLowerCase().includes(q);
}

/**
 * 命令面板过滤：按 i18n 名称 / id 过滤导航项
 * labelOf 为 i18n 翻译函数（测试时可传恒等函数）
 */
export function filterNavItems(query: string, labelOf: (labelKey: string) => string): NavItem[] {
  const q = query.trim().toLowerCase();
  if (!q) return flattenNavItems();
  return flattenNavItems().filter(
    (i) => fuzzyMatch(q, labelOf(i.label)) || fuzzyMatch(q, i.id)
  );
}
