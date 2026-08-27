/**
 * AI 助手文件系统工具：列目录 / 读文本 / 新建文件 / 覆盖写文件
 *
 * 设计约束：
 * - read_text_file 仅支持 UTF-8 文本；PDF/DOCX 等文档请引导 LLM 用 pdf_extract_text
 * - create_file 只新建不覆盖（安全）；write_file 可覆盖但标记 dangerous，执行前需用户确认
 */
import { listDir, readTextFile, writeTextFile } from "../../api";
import type { AiTool } from "./types";
import { strArg } from "./utils";

/** 回传 LLM 的内容上限（字符），防止撑爆模型上下文 */
const MAX_PREVIEW_CHARS = 20_000;
/** 目录列表条目上限 */
const MAX_ENTRIES = 200;

/** 单层列出目录内容 */
const listDirectoryTool: AiTool = {
  name: "list_directory",
  description:
    "列出某个目录下的文件和子文件夹（单层，不递归；隐藏文件已跳过，目录排在前面）。" +
    "用于浏览用户文件结构、寻找文件。需要递归按扩展名找文件时，结果超过单层可用多次调用。",
  parameters: {
    type: "object",
    properties: {
      path: { type: "string", description: "目录的绝对路径" },
    },
    required: ["path"],
    additionalProperties: false,
  },
  async execute(args) {
    const dir = strArg(args, "path");
    if (!dir) return { ok: false, message: "参数错误：缺少 path" };
    const entries = await listDir(dir);
    if (entries.length === 0) return { ok: true, message: "目录为空" };
    const shown = entries.slice(0, MAX_ENTRIES);
    const lines = shown.map((e) =>
      e.isDir ? `[目录] ${e.name}` : `${e.name}（${e.size} 字节）`,
    );
    const more = entries.length > MAX_ENTRIES ? `\n…共 ${entries.length} 个条目，仅显示前 ${MAX_ENTRIES} 个` : "";
    return { ok: true, message: `目录 ${dir} 内容：\n${lines.join("\n")}${more}` };
  },
};

/** 读取 UTF-8 文本文件内容 */
const readTextFileTool: AiTool = {
  name: "read_text_file",
  description:
    "读取 UTF-8 文本文件的内容（txt / md / csv / json / 源代码等），用于了解文件内容。" +
    "注意：PDF、DOCX 等文档格式请改用 pdf_extract_text 工具。内容过长时只返回前 2 万字符。",
  parameters: {
    type: "object",
    properties: {
      path: { type: "string", description: "文件的绝对路径" },
    },
    required: ["path"],
    additionalProperties: false,
  },
  async execute(args) {
    const path = strArg(args, "path");
    if (!path) return { ok: false, message: "参数错误：缺少 path" };
    const r = await readTextFile(path);
    if (!r.content.trim()) return { ok: true, message: `文件为空：${path}` };
    const preview = r.content.slice(0, MAX_PREVIEW_CHARS);
    const truncatedNote =
      r.truncated || r.content.length > MAX_PREVIEW_CHARS
        ? `（内容过长已截断：全文 ${r.totalBytes} 字节）`
        : "";
    return { ok: true, message: `文件 ${path} 的内容${truncatedNote}：\n\n${preview}` };
  },
};

/** 新建文本文件（不覆盖已有文件） */
const createFileTool: AiTool = {
  name: "create_file",
  description:
    "创建一个新的 UTF-8 文本文件并写入内容（如 .txt / .md / .csv / .json / 源代码）。" +
    "自动创建缺失的父目录；若文件已存在会报错（此时应换一个文件名，或改用 write_file 工具并征得用户同意）。",
  parameters: {
    type: "object",
    properties: {
      path: { type: "string", description: "要创建的文件绝对路径（含文件名）" },
      content: { type: "string", description: "要写入的完整文本内容" },
    },
    required: ["path", "content"],
    additionalProperties: false,
  },
  async execute(args) {
    const path = strArg(args, "path");
    const content = typeof args.content === "string" ? args.content : undefined;
    if (!path || content === undefined) {
      return { ok: false, message: "参数错误：缺少 path 或 content" };
    }
    const out = await writeTextFile(path, content, false);
    return { ok: true, message: `文件已创建：${out}`, outputs: [out] };
  },
};

/** 覆盖写入文本文件（危险：需用户确认） */
const writeFileTool: AiTool = {
  name: "write_file",
  description:
    "向文件写入内容，允许覆盖已有文件（危险操作，执行前会请求用户确认）。" +
    "新建文件请优先使用 create_file 工具；仅在用户明确要求修改/覆盖某个文件时使用本工具。",
  dangerous: true,
  parameters: {
    type: "object",
    properties: {
      path: { type: "string", description: "文件的绝对路径（含文件名）" },
      content: { type: "string", description: "要写入的完整文本内容" },
    },
    required: ["path", "content"],
    additionalProperties: false,
  },
  async execute(args, ctx) {
    const path = strArg(args, "path");
    const content = typeof args.content === "string" ? args.content : undefined;
    if (!path || content === undefined) {
      return { ok: false, message: "参数错误：缺少 path 或 content" };
    }
    const ok = await ctx.confirm(`AI 请求覆盖写入文件：${path}`);
    if (!ok) return { ok: false, message: "用户取消了写入操作" };
    const out = await writeTextFile(path, content, true);
    return { ok: true, message: `文件已写入：${out}`, outputs: [out] };
  },
};

/** 文件系统工具注册表 */
export const fsTools: AiTool[] = [listDirectoryTool, readTextFileTool, createFileTool, writeFileTool];
