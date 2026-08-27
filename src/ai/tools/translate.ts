/**
 * AI 翻译工具：整篇文档翻译为目标语言，输出 Markdown / TXT / DOCX
 *
 * 流程：extractText 提取原文 → 按段落边界分块 → 逐块调用 chat 翻译（保留
 * 段落结构与 Markdown 标记）→ 拼接 → writeTextFile 输出到源文件同目录。
 */
import { convertDocument, extractText, writeTextFile } from "../../api";
import { chat } from "../index";
import { basename, dirname, join } from "@tauri-apps/api/path";
import type { ChatMessage } from "../types";
import type { AiTool } from "./types";
import { chunkText, langCode, stripFences } from "./translate-utils";
import { stamp, strArg } from "./utils";

/** 单块翻译失败重试次数 */
const MAX_ATTEMPTS = 3;
/** 分块数上限（约 20 万字符），防止失控的 token 成本 */
const MAX_CHUNKS = 80;

/** 翻译系统提示词 */
function translateSystemPrompt(targetLang: string): string {
  return (
    `你是专业翻译引擎。将用户消息中的文本翻译为${targetLang}，严格遵守：\n` +
    "1. 只输出译文本身，不要任何解释、前言或代码围栏。\n" +
    "2. 保持原文的段落划分与空行结构一一对应。\n" +
    "3. 保留 Markdown 标记（# 标题、- 列表、**加粗**、*斜体*、`行内代码`），代码块内容原样保留不翻译。\n" +
    "4. 不翻译 URL、HTML 标签、命令、变量名等技术片段。\n" +
    "5. 译文自然流畅，符合目标语言的表达习惯。"
  );
}

/** 单块翻译（含重试） */
async function translateChunk(chunk: string, targetLang: string): Promise<string> {
  const messages: ChatMessage[] = [
    { role: "system", content: translateSystemPrompt(targetLang) },
    { role: "user", content: chunk },
  ];
  let lastErr = "";
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    try {
      const out = await chat(messages);
      const cleaned = stripFences(out);
      if (cleaned.trim()) return cleaned;
      lastErr = "模型返回了空内容";
    } catch (e: any) {
      lastErr = String(e || "未知错误");
    }
  }
  throw new Error(lastErr);
}

/** 整篇文档翻译 */
const translateDocumentTool: AiTool = {
  name: "translate_document",
  description:
    "把整篇文档翻译为目标语言并输出新文件（默认 Markdown，可选 txt/docx）。" +
    "支持 PDF / Word / TXT / MD / HTML 等可提取文本的格式；长文自动分块翻译，保留段落结构与 Markdown 格式；" +
    "输出文件生成在源文件所在目录（文件名：原名_语言缩写.扩展名）。需要 AI 引擎已配置（设置 → AI 能力）。",
  parameters: {
    type: "object",
    properties: {
      input_path: { type: "string", description: "源文档的绝对路径" },
      target_lang: { type: "string", description: "目标语言，如 English、日语、中文、fr" },
      output_ext: {
        type: "string",
        enum: ["md", "txt", "docx"],
        description: "输出文件格式（默认 md）",
      },
    },
    required: ["input_path", "target_lang"],
    additionalProperties: false,
  },
  async execute(args) {
    const input = strArg(args, "input_path");
    const targetLang = strArg(args, "target_lang");
    const outputExt = strArg(args, "output_ext") ?? "md";
    if (!input || !targetLang) {
      return { ok: false, message: "参数错误：缺少 input_path 或 target_lang" };
    }
    if (!["md", "txt", "docx"].includes(outputExt)) {
      return { ok: false, message: `参数错误：output_ext 只能是 md / txt / docx，收到 ${outputExt}` };
    }

    // 1. 提取原文
    let raw: string;
    try {
      raw = await extractText(input);
    } catch (e: any) {
      return { ok: false, message: `提取文档文本失败：${String(e)}` };
    }
    if (!raw.trim()) {
      return { ok: false, message: "文档中没有可翻译的文本内容（可能是扫描件）" };
    }

    // 2. 分块
    const chunks = chunkText(raw);
    if (chunks.length > MAX_CHUNKS) {
      return {
        ok: false,
        message: `文档过长（约 ${raw.length} 字符，需 ${chunks.length} 块 > 上限 ${MAX_CHUNKS} 块），请拆分后再翻译`,
      };
    }

    // 3. 逐块翻译
    const translated: string[] = [];
    for (let i = 0; i < chunks.length; i++) {
      try {
        translated.push(await translateChunk(chunks[i], targetLang));
      } catch (e: any) {
        return {
          ok: false,
          message: `第 ${i + 1}/${chunks.length} 块翻译失败：${String(e)}`,
        };
      }
    }
    const result = translated.join("\n\n");

    // 4. 写出译文（md/txt 直接写；docx 先写 md 再转换；目标名冲突时回退加时间戳后缀）
    const dir = await dirname(input);
    const base = await basename(input);
    const stem = base.includes(".") ? base.slice(0, base.lastIndexOf(".")) : base;
    const code = langCode(targetLang);
    const writeExt = outputExt === "docx" ? "md" : outputExt;
    let outPath = await join(dir, `${stem}_${code}.${writeExt}`);
    try {
      outPath = await writeTextFile(outPath, result, false);
    } catch {
      outPath = await writeTextFile(await join(dir, `${stem}_${code}_${stamp()}.${writeExt}`), result, false);
    }

    // 5. docx 输出：md → docx（内置引擎），失败时保留 md 版本
    if (outputExt === "docx") {
      try {
        const finalPath = await convertDocument(outPath, "docx", dir, "builtin");
        return {
          ok: true,
          message: `翻译完成（${targetLang}，${chunks.length} 块），输出文件：${finalPath}`,
          outputs: [finalPath],
        };
      } catch (e: any) {
        return {
          ok: true,
          message: `翻译完成但 docx 转换失败（${String(e)}），已保留 Markdown 版本，输出文件：${outPath}`,
          outputs: [outPath],
        };
      }
    }

    return {
      ok: true,
      message: `翻译完成（${targetLang}，${chunks.length} 块，${result.length} 字符），输出文件：${outPath}`,
      outputs: [outPath],
    };
  },
};

/** 翻译工具注册表 */
export const translateTools: AiTool[] = [translateDocumentTool];
