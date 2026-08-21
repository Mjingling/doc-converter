import {
  getPdfPageCount, pdfCompress, pdfDecrypt, pdfDeletePages, pdfEncrypt, pdfExtractImages,
  pdfExtractPages, pdfExtractText, pdfMerge, pdfPageNumbers, pdfRotate, pdfSplit, pdfWatermark,
} from "../../api";
import type { AiTool } from "./types";
import { numArg, outDirFor, outputPathFor, strArg } from "./utils";

/** 公共输入路径参数 Schema */
const inputSchema = {
  input_path: { type: "string", description: "输入 PDF 文件的绝对路径" },
};

/** PDF 合并 */
const pdfMergeTool: AiTool = {
  name: "pdf_merge",
  description:
    "合并多个 PDF 文件为一个 PDF。输出文件自动生成在第一个输入文件所在目录，成功时返回输出路径。",
  parameters: {
    type: "object",
    properties: {
      paths: {
        type: "array",
        items: { type: "string", description: "待合并 PDF 的绝对路径" },
        minItems: 2,
        description: "按此顺序合并的 PDF 文件绝对路径列表",
      },
    },
    required: ["paths"],
    additionalProperties: false,
  },
  async execute(args) {
    const paths = args.paths;
    if (!Array.isArray(paths) || paths.length < 2 || !paths.every((p) => typeof p === "string")) {
      return { ok: false, message: "参数错误：paths 需要至少两个文件绝对路径" };
    }
    const outPath = await outputPathFor(paths[0] as string, "merged");
    const result = await pdfMerge(paths as string[], outPath);
    return { ok: true, message: `合并完成，输出文件：${result}` };
  },
};

/** PDF 拆分（按页码范围） */
const pdfSplitTool: AiTool = {
  name: "pdf_split",
  description:
    "按页码范围拆分 PDF（如 [[1,5],[6,10]] 拆成两份）。页码范围 1-based 且闭合，不可重叠。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      ranges: {
        type: "array",
        items: {
          type: "array",
          items: { type: "integer" },
          minItems: 2,
          maxItems: 2,
        },
        description: "拆分范围列表，如 [[1,5],[6,10]] 表示第 1-5 页与第 6-10 页各为一份",
      },
    },
    required: ["input_path", "ranges"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const ranges = args.ranges;
    if (!Array.isArray(ranges) || ranges.length === 0) {
      return { ok: false, message: "参数错误：ranges 需要至少一个 [start, end] 范围" };
    }
    const parsed = ranges.map((r) => {
      if (!Array.isArray(r) || r.length !== 2 || r.some((x) => typeof x !== "number")) {
        throw new Error("ranges 元素必须是 [start, end] 数字对");
      }
      return [r[0] as number, r[1] as number] as [number, number];
    });
    const dir = await outDirFor(input, "split");
    const outputs = await pdfSplit(input, parsed, dir);
    return { ok: true, message: `拆分完成，共 ${outputs.length} 份：\n${outputs.join("\n")}`, outputs };
  },
};

/** PDF 压缩 */
const pdfCompressTool: AiTool = {
  name: "pdf_compress",
  description: "压缩 PDF 文件体积（重新编码内部图片）。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: { input_path: inputSchema.input_path },
    required: ["input_path"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const outPath = await outputPathFor(input, "compressed");
    const result = await pdfCompress(input, outPath);
    return { ok: true, message: `压缩完成，输出文件：${result}` };
  },
};

/** 提取指定页面 */
const pdfExtractPagesTool: AiTool = {
  name: "pdf_extract_pages",
  description:
    "从 PDF 中提取指定页面（页码 1-based，任意顺序，可重复）生成新 PDF。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      pages: {
        type: "array",
        items: { type: "integer", description: "1-based 页码" },
        minItems: 1,
        description: "要提取的页码列表，如 [1,3,5]",
      },
    },
    required: ["input_path", "pages"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const pages = args.pages;
    if (!Array.isArray(pages) || pages.length === 0 || pages.some((p) => typeof p !== "number")) {
      return { ok: false, message: "参数错误：pages 需要非空页码数字列表" };
    }
    const outPath = await outputPathFor(input, `pages_${(pages as number[]).join("_")}`);
    const result = await pdfExtractPages(input, outPath, pages as number[]);
    return { ok: true, message: `页面提取完成，输出文件：${result}` };
  },
};

/** 删除指定范围页面 */
const pdfDeletePagesTool: AiTool = {
  name: "pdf_delete_pages",
  description:
    "删除 PDF 中指定范围的页面（1-based 闭合区间，如 [[2,3]] 删除第 2-3 页）。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      ranges: {
        type: "array",
        items: {
          type: "array",
          items: { type: "integer" },
          minItems: 2,
          maxItems: 2,
        },
        description: "要删除的页码范围列表，如 [[2,3]]",
      },
    },
    required: ["input_path", "ranges"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const ranges = args.ranges;
    if (!Array.isArray(ranges) || ranges.length === 0) {
      return { ok: false, message: "参数错误：ranges 需要至少一个 [start, end] 范围" };
    }
    const parsed = ranges.map((r) => {
      if (!Array.isArray(r) || r.length !== 2 || r.some((x) => typeof x !== "number")) {
        throw new Error("ranges 元素必须是 [start, end] 数字对");
      }
      return [r[0] as number, r[1] as number] as [number, number];
    });
    const outPath = await outputPathFor(input, "deleted");
    const result = await pdfDeletePages(input, outPath, parsed);
    return { ok: true, message: `页面删除完成，输出文件：${result}` };
  },
};

/** 旋转 */
const pdfRotateTool: AiTool = {
  name: "pdf_rotate",
  description: "旋转 PDF 全部页面（90 / 180 / 270 度顺时针）。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      angle: { type: "integer", enum: [90, 180, 270], description: "顺时针旋转角度" },
    },
    required: ["input_path", "angle"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const angle = numArg(args, "angle");
    if (angle === undefined || ![90, 180, 270].includes(angle)) {
      return { ok: false, message: "参数错误：angle 只能是 90 / 180 / 270" };
    }
    const outPath = await outputPathFor(input, `rotated_${angle}`);
    const result = await pdfRotate(input, outPath, angle);
    return { ok: true, message: `旋转完成，输出文件：${result}` };
  },
};

/** 添加水印 */
const pdfWatermarkTool: AiTool = {
  name: "pdf_watermark",
  description:
    "给 PDF 全部页面添加平铺文字水印。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      text: { type: "string", description: "水印文字，如「机密」" },
      opacity: { type: "number", minimum: 0.05, maximum: 1, description: "透明度（默认 0.3）" },
      color: {
        type: "array",
        items: { type: "integer", minimum: 0, maximum: 255 },
        minItems: 3,
        maxItems: 3,
        description: "RGB 颜色（0-255），如 [200, 0, 0] 为红色（默认灰色）",
      },
      font_size: { type: "number", description: "字号，单位 pt（默认 48）" },
    },
    required: ["input_path", "text"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    const text = strArg(args, "text");
    if (!input || !text) return { ok: false, message: "参数错误：缺少 input_path 或 text" };
    let color: [number, number, number] = [128, 128, 128];
    const c = args.color;
    if (Array.isArray(c) && c.length === 3 && c.every((x) => typeof x === "number")) {
      color = [c[0], c[1], c[2]] as [number, number, number];
    }
    const outPath = await outputPathFor(input, "watermark");
    const result = await pdfWatermark(
      input, outPath, text,
      numArg(args, "opacity", 0.3)!,
      color,
      numArg(args, "font_size", 48)!,
    );
    return { ok: true, message: `水印添加完成，输出文件：${result}` };
  },
};

/** 添加页码 */
const pdfPageNumbersTool: AiTool = {
  name: "pdf_page_numbers",
  description: "给 PDF 全部页面添加页码。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      style: {
        type: "string",
        enum: ["page", "pageOf"],
        description: "page = 仅页码；pageOf = 页码/总页数（默认 page）",
      },
    },
    required: ["input_path"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const style = strArg(args, "style") === "pageOf" ? "pageOf" : "page";
    const outPath = await outputPathFor(input, "pagenum");
    const result = await pdfPageNumbers(input, outPath, style);
    return { ok: true, message: `页码添加完成，输出文件：${result}` };
  },
};

/** 加密 */
const pdfEncryptTool: AiTool = {
  name: "pdf_encrypt",
  description:
    "加密 PDF（设置打开密码；RC4-128 加密）。输出文件自动生成在输入文件所在目录。请提醒用户牢记密码。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      user_password: { type: "string", description: "打开 PDF 所需的用户密码" },
      owner_password: { type: "string", description: "所有者密码（可选，默认与用户密码相同）" },
    },
    required: ["input_path", "user_password"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    const pass = strArg(args, "user_password");
    if (!input || !pass) return { ok: false, message: "参数错误：缺少 input_path 或 user_password" };
    const outPath = await outputPathFor(input, "encrypted");
    const result = await pdfEncrypt(input, outPath, pass, strArg(args, "owner_password") ?? pass);
    return { ok: true, message: `加密完成，输出文件：${result}` };
  },
};

/** 解密 */
const pdfDecryptTool: AiTool = {
  name: "pdf_decrypt",
  description: "移除 PDF 的打开密码（需要提供当前密码）。输出文件自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: {
      input_path: inputSchema.input_path,
      password: { type: "string", description: "当前打开密码" },
    },
    required: ["input_path", "password"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    const pass = strArg(args, "password");
    if (!input || !pass) return { ok: false, message: "参数错误：缺少 input_path 或 password" };
    const outPath = await outputPathFor(input, "decrypted");
    const result = await pdfDecrypt(input, outPath, pass);
    return { ok: true, message: `解密完成，输出文件：${result}` };
  },
};

/** 提取文本 */
const pdfExtractTextTool: AiTool = {
  name: "pdf_extract_text",
  description:
    "提取 PDF 全文文本。返回文本前 2000 字符与总字符数，供你了解文档内容以决定后续操作。",
  parameters: {
    type: "object",
    properties: { input_path: inputSchema.input_path },
    required: ["input_path"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const text = await pdfExtractText(input);
    if (!text.trim()) return { ok: true, message: "该 PDF 无可提取文本（可能为扫描件）" };
    const preview = text.slice(0, 2000);
    return {
      ok: true,
      message: `提取到 ${text.length} 字符。内容预览（前 2000 字符）：\n${preview}`,
    };
  },
};

/** 提取 PDF 内嵌图片 */
const pdfExtractImagesTool: AiTool = {
  name: "pdf_extract_images",
  description: "提取 PDF 中嵌入的全部图片到输出目录。输出目录自动生成在输入文件所在目录。",
  parameters: {
    type: "object",
    properties: { input_path: inputSchema.input_path },
    required: ["input_path"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const dir = await outDirFor(input, "images");
    const outputs = await pdfExtractImages(input, dir);
    return { ok: true, message: `提取到 ${outputs.length} 张图片，输出目录：${dir}` };
  },
};

/** 获取 PDF 页数 */
const pdfPageCountTool: AiTool = {
  name: "pdf_page_count",
  description: "获取 PDF 总页数。在拆分/提取页面前先用它确认页码范围。",
  parameters: {
    type: "object",
    properties: { input_path: inputSchema.input_path },
    required: ["input_path"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    if (!input) return { ok: false, message: "参数错误：缺少 input_path" };
    const n = await getPdfPageCount(input);
    return { ok: true, message: `该 PDF 共 ${n} 页` };
  },
};

export const pdfTools: AiTool[] = [
  pdfMergeTool,
  pdfSplitTool,
  pdfCompressTool,
  pdfExtractPagesTool,
  pdfDeletePagesTool,
  pdfRotateTool,
  pdfWatermarkTool,
  pdfPageNumbersTool,
  pdfEncryptTool,
  pdfDecryptTool,
  pdfExtractTextTool,
  pdfExtractImagesTool,
  pdfPageCountTool,
];
