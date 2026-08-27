import type { ToolCall } from "../types";
import { docTools } from "./doc";
import { fsTools } from "./fs";
import { pdfTools } from "./pdf";
import { translateTools } from "./translate";
import type { AiTool, ToolContext, ToolResult } from "./types";
import { toToolDefinition } from "./types";

/** 全部可调度工具（AI 助手 function calling 注册表） */
export const AI_TOOLS: AiTool[] = [...pdfTools, ...docTools, ...fsTools, ...translateTools];

/** 工具名 → 工具 查找表 */
const toolByName = new Map(AI_TOOLS.map((t) => [t.name, t]));

/** OpenAI 兼容的 tools 定义列表（随 chat 请求发送给模型） */
export const TOOL_DEFINITIONS = AI_TOOLS.map(toToolDefinition);

/** 根据工具名查找工具 */
export function findTool(name: string): AiTool | undefined {
  return toolByName.get(name);
}

/** 从工具结果 message 提取输出文件/目录路径（各工具统一使用“输出文件：/输出目录：”格式） */
function extractOutputs(msg: string): string[] | undefined {
  const m = msg.match(/(?:输出文件|输出目录|Output file|Output dir(?:ectory)?)[:：]\s*(\S+)/);
  return m ? [m[1]] : undefined;
}

/** 执行工具调用（解析 LLM 返回的 JSON 参数并执行对应 Tauri 命令） */
export async function executeTool(call: ToolCall, ctx: ToolContext): Promise<ToolResult> {
  const tool = toolByName.get(call.name);
  if (!tool) return { ok: false, message: `未知工具：${call.name}` };
  let args: Record<string, unknown> = {};
  try {
    args = JSON.parse(call.arguments || "{}");
  } catch {
    return { ok: false, message: `工具 ${call.name} 参数不是合法 JSON：${call.arguments}` };
  }
  if (typeof args !== "object" || args === null || Array.isArray(args)) {
    return { ok: false, message: `工具 ${call.name} 参数格式异常` };
  }
  try {
    const res = await tool.execute(args, ctx);
    // 统一提取输出路径，供界面展示“打开文件/打开目录”
    if (res.ok && !res.outputs) res.outputs = extractOutputs(res.message);
    return res;
  } catch (e: any) {
    return { ok: false, message: `工具 ${call.name} 执行失败：${String(e || "未知错误")}` };
  }
}

export type { AiTool, ToolContext, ToolResult } from "./types";
