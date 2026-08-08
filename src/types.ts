/** 转换引擎状态 */
export interface EngineStatus {
  available: boolean;
  path: string | null;
}

/** 引擎模式：内置引擎（PDF 处理，零外部依赖） / LibreOffice 引擎（文档转换，需自行安装） */
export type EngineMode = "builtin" | "libreoffice";

/** 左侧导航功能项 ID */
export type NavId =
  | "merge" | "split" | "compress" | "organize" // PDF 处理（内置引擎）
  | "watermark" | "rotate" | "encrypt" | "images2pdf" | "batch" // PDF 工具箱（内置引擎）
  | "metadata" | "crop" | "outline" | "docxExtract" | "imageCompress" // 批次 B/C 扩展
  | "history" // 历史记录
  | "pdf2word" | "pdf2image" | "word2pdf" // 文档转换（LibreOffice）
  | "excel2pdf" | "ppt2pdf" | "convert";

/** 目标格式 */
export interface ConvertTarget {
  ext: string;
  /** i18n key（如 convert.targetPdf），渲染时通过 t() 解析 */
  label: string;
}

/** 转换场景配置：左侧导航中每个转换功能页的配置 */
export interface ConvertScene {
  /** i18n key：页面标题（如 scenes.pdf2word.title） */
  title: string;
  /** i18n key：副标题说明 */
  subtitle: string;
  /** 可接受的输入扩展名（不含点） */
  acceptExts: string[];
  /** 静态目标格式；为空时上传文件后从后端动态获取 */
  fixedTargets?: ConvertTarget[];
  /** 是否依赖 LibreOffice 引擎 */
  engineRequired: boolean;
}

/** 格式信息 */
export interface FormatInfo {
  ext: string;
  label: string;
  targets: FormatInfo[];
}
