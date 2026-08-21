import { batchRename, convertDocument, docxExtractImages, imagesToPdf, webpageToPdf } from "../../api";
import { useEngineStore } from "../../stores/engine";
import { useSettingsStore } from "../../stores/settings";
import { dirname, homeDir, join } from "@tauri-apps/api/path";
import type { AiTool } from "./types";
import { outDirFor, outputPathFor, stamp, strArg, strArrArg } from "./utils";

/** 文档格式转换（走 LibreOffice / 内置引擎） */
const convertDocumentTool: AiTool = {
  name: "convert_document",
  description:
    "文档格式转换：PDF/Word/Excel/PPT/文本/图片等互转。target_ext 为目标扩展名（如 pdf、docx、xlsx、pptx、txt、md、html、csv、png、jpg）。" +
    "文档互转（如 PDF↔Word/Excel/PPT）需要 LibreOffice 引擎；engine 参数默认 auto（跟随应用当前引擎选择），" +
    "若转换失败会返回错误，可提示用户切换到 LibreOffice 引擎或安装 LibreOffice。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: { type: "string", description: "输入文件的绝对路径" },
      target_ext: { type: "string", description: "目标扩展名（不含点），如 docx、pdf" },
      engine: {
        type: "string",
        enum: ["auto", "builtin", "libreoffice"],
        description: "转换引擎：auto 跟随应用当前选择（默认）；文档互转通常需要 libreoffice",
      },
    },
    required: ["input_path", "target_ext"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    const target = strArg(args, "target_ext");
    if (!input || !target) return { ok: false, message: "参数错误：缺少 input_path 或 target_ext" };
    const engine = strArg(args, "engine") ?? "auto";
    if (!["auto", "builtin", "libreoffice"].includes(engine)) {
      return { ok: false, message: `参数错误：engine 只能是 auto / builtin / libreoffice，收到 ${engine}` };
    }
    const mode = engine === "auto" ? useEngineStore().mode : (engine as "builtin" | "libreoffice");
    const outPath = await outputPathFor(input, "converted", `.${target}`);
    const result = await convertDocument(input, target, await dirname(input), mode);
    return { ok: true, message: `转换完成（${target.toUpperCase()}），输出文件：${result}` };
  },
};

/** 网页转 PDF */
const webpageToPdfTool: AiTool = {
  name: "webpage_to_pdf",
  description:
    "将网页 URL 保存为 PDF（等待页面渲染完成）。输出文件自动生成在应用默认输出目录（未设置时为用户下载目录）。",
  parameters: {
    type: "object",
    properties: {
      url: { type: "string", description: "网页完整 URL，如 https://example.com/page" },
    },
    required: ["url"],
    additionalProperties: false,
  },
  async execute(args) {
    const url = strArg(args, "url");
    if (!url) return { ok: false, message: "参数错误：缺少 url" };
    const settings = useSettingsStore();
    const dir = settings.defaultOutDir || (await homeDir());
    const host = (() => {
      try {
        return new URL(url).hostname.replace(/[^a-z0-9]/gi, "_");
      } catch {
        return "webpage";
      }
    })();
    const outPath = await join(dir, `${host}_${stamp()}.pdf`);
    const result = await webpageToPdf(url, outPath);
    return { ok: true, message: `网页已保存为 PDF，输出文件：${result}` };
  },
};

/** 多张图片合成 PDF */
const imagesToPdfTool: AiTool = {
  name: "images_to_pdf",
  description: "将多张图片（png/jpg/jpeg/bmp/gif/webp）按顺序合成一个 PDF。输出文件自动生成在第一张图片所在目录。",
  parameters: {
    type: "object",
    properties: {
      paths: {
        type: "array",
        items: { type: "string", description: "图片文件绝对路径" },
        minItems: 1,
        description: "按此顺序合成的图片绝对路径列表",
      },
      page_size: {
        type: "string",
        enum: ["auto", "a4"],
        description: "页面尺寸：auto 跟随图片尺寸（默认）；a4 为 A4 居中",
      },
    },
    required: ["paths"],
    additionalProperties: false,
  },
  async execute(args) {
    const paths = strArrArg(args, "paths");
    if (!paths) return { ok: false, message: "参数错误：paths 需要非空图片路径列表" };
    const pageSize = strArg(args, "page_size") === "a4" ? "a4" : "auto";
    const outPath = await outputPathFor(paths[0], "images2pdf", ".pdf");
    const result = await imagesToPdf(paths, outPath, pageSize);
    return { ok: true, message: `图片已合成 PDF，输出文件：${result}` };
  },
};

/** 提取 docx 内嵌图片 */
const docxExtractImagesTool: AiTool = {
  name: "docx_extract_images",
  description: "提取 Word（docx）文档中嵌入的全部图片到输出目录。输出目录自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: { input_path: { type: "string", description: "输入 docx 文件的绝对路径" } },
    required: ["input_path"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const dir = await outDirFor(input, "images");
    const outputs = await docxExtractImages(input, dir);
    return { ok: true, message: `提取到 ${outputs.length} 张图片，输出目录：${dir}` };
  },
};

/** 批量重命名（危险：执行前需用户确认） */
const batchRenameTool: AiTool = {
  name: "batch_rename",
  description:
    "批量重命名文件。items 为 [旧绝对路径, 新绝对路径] 对列表。此操作会直接修改用户文件，执行前必须请求用户确认。",
  dangerous: true,
  parameters: {
    type: "object",
    properties: {
      items: {
        type: "array",
        items: {
          type: "array",
          items: { type: "string" },
          minItems: 2,
          maxItems: 2,
          description: "[旧绝对路径, 新绝对路径]",
        },
        minItems: 1,
        description: "重命名规则列表",
      },
    },
    required: ["items"],
    additionalProperties: false,
  },
  async execute(args, ctx) {
    const items = args.items;
    if (!Array.isArray(items) || items.length === 0 || items.some((it) => !Array.isArray(it) || it.length !== 2 || it.some((x) => typeof x !== "string"))) {
      return { ok: false, message: "参数错误：items 需要 [旧路径, 新路径] 对列表" };
    }
    const pairs = items as [string, string][];
    const ok = await ctx.confirm(`批量重命名 ${pairs.length} 个文件（${pairs.map((p) => p[1].split(/[/\\]/).pop()).join("、")}）`);
    if (!ok) return { ok: false, message: "用户取消了重命名操作" };
    const results = await batchRename(pairs);
    const failed = results.filter((r) => !r.ok);
    if (failed.length > 0) {
      return {
        ok: false,
        message: `${failed.length}/${results.length} 个重命名失败：${failed.map((r) => `${r.old_path} → ${r.error}`).join("；")}`,
      };
    }
    return { ok: true, message: `已重命名 ${results.length} 个文件` };
  },
};

export const docTools: AiTool[] = [
  convertDocumentTool,
  webpageToPdfTool,
  imagesToPdfTool,
  docxExtractImagesTool,
  batchRenameTool,
];
