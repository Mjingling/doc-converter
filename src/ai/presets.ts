/**
 * 云端 AI 服务商预设：一键填充 OpenAI 兼容 API 配置
 * （API 密钥由用户在各平台自行申请，预设不包含密钥）
 */
export interface CloudAiPreset {
  /** 预设标识（用于选中态判断） */
  id: string;
  /** 显示名文案 key（i18n settings 段） */
  labelKey: string;
  /** API 地址（OpenAI 兼容端点） */
  baseUrl: string;
  /** 对话模型名 */
  chatModel: string;
  /** Embedding 模型名 */
  embeddingModel: string;
}

/** 内置服务商预设：智谱 BigModel（chat 免费）与 OpenAI */
export const CLOUD_AI_PRESETS: CloudAiPreset[] = [
  {
    id: "zhipu",
    labelKey: "settings.aiPresetZhipu",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    chatModel: "glm-4-flash-250414",
    embeddingModel: "embedding-3",
  },
  {
    id: "openai",
    labelKey: "settings.aiPresetOpenAI",
    baseUrl: "https://api.openai.com/v1",
    chatModel: "gpt-4o-mini",
    embeddingModel: "text-embedding-3-small",
  },
];
