/**
 * AI 工具（function calling）注册表类型定义：
 * 把 Tauri 命令包装为 LLM 可调用的工具（名称 + 描述 + JSON Schema 参数 + 执行函数）
 */

/** 工具执行结果（文本形式回传给 LLM） */
export interface ToolResult {
  ok: boolean;
  /** 回传给 LLM 的文本：成功时列出输出文件，失败时为错误信息 */
  message: string;
  /** 生成的输出文件路径（供界面展示“打开文件/目录”；未填时由 executeTool 从 message 提取） */
  outputs?: string[];
}

/** 工具执行上下文 */
export interface ToolContext {
  /** 危险操作（覆盖/重命名等）执行前请求用户确认；拒绝时返回 false */
  confirm: (description: string) => Promise<boolean>;
}

/** AI 可调用工具定义 */
export interface AiTool {
  /** 工具名（LLM 调用时使用，蛇形命名） */
  name: string;
  /** 工具描述（LLM 理解用途与约束） */
  description: string;
  /** 参数 JSON Schema（OpenAI tools.parameters 格式） */
  parameters: Record<string, unknown>;
  /** 执行前是否需要用户确认（如覆盖文件、重命名） */
  dangerous?: boolean;
  execute(args: Record<string, unknown>, ctx: ToolContext): Promise<ToolResult>;
}

/** 从 AiTool 派生 OpenAI 兼容的 tools 定义 */
export function toToolDefinition(tool: AiTool) {
  return {
    type: "function" as const,
    function: {
      name: tool.name,
      description: tool.description,
      parameters: tool.parameters,
    },
  };
}
