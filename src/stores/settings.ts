import { defineStore } from "pinia";
import { load as loadStore, type Store } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";
import { setPlatformDefaultDir } from "../utils/file";

/** 持久化文件名（位于应用数据目录，macOS: ~/Library/Application Support/<identifier>/） */
const FILE = "settings.json";

export type AppLocale = "system" | "zh-CN" | "en-US" | "ja-JP" | "ko-KR";
export type AppTheme = "system" | "light" | "dark";

export type AiMode = "auto" | "local" | "local-server" | "cloud";

/** 云端 AI 配置（OpenAI 兼容 API） */
export interface CloudAiConfig {
  /** API 地址，如 https://api.openai.com/v1 */
  baseUrl: string;
  /** API 密钥 */
  apiKey: string;
  /** embedding 模型名 */
  embeddingModel: string;
  /** 对话模型名 */
  chatModel: string;
}

/** 本地服务 AI 配置（Ollama / LM Studio 等 OpenAI 兼容端点） */
export interface LocalServerAiConfig {
  /** 服务地址，如 http://localhost:11434/v1 */
  baseUrl: string;
  /** 对话模型名（留空由服务决定） */
  chatModel: string;
  /** Embedding 模型名（留空由服务决定） */
  embeddingModel: string;
}

/** 搜索提供商类型 */
export type SearchProvider = "off" | "zhipu" | "tavily";

/** 网页搜索配置：zhipu 复用云端密钥；tavily 用独立免费 key */
export interface AiSearchConfig {
  provider: SearchProvider;
  tavilyKey: string;
}

/** AI 能力配置：mode 引擎模式，cloud 云端 API 参数，localServer 本地服务参数，search 网页搜索，localChatModelId 本地生成式模型 */
export interface AiConfig {
  mode: AiMode;
  cloud: CloudAiConfig;
  localServer: LocalServerAiConfig;
  /** 网页搜索（AI 助手实时信息查询） */
  search: AiSearchConfig;
  /** 本地生成式模型（chat）HuggingFace 模型 ID，如 Qwen/Qwen2.5-0.5B-Instruct */
  localChatModelId: string;
}

/** 文件夹监控配置：enabled 开关、folder 监控目录、targets 格式规则（扩展名 → 目标扩展名） */
export interface WatcherConfig {
  enabled: boolean;
  folder: string;
  targets: Record<string, string>;
}

/** 桌面宠物配置：enabled 开关（右下角透明置顶小窗口） */
export interface PetConfig {
  enabled: boolean;
}

interface SettingsState {
  /** 默认输出目录（空字符串 = 输出到输入文件所在目录） */
  defaultOutDir: string;
  /** 界面语言（system = 跟随系统） */
  locale: AppLocale;
  /** 界面主题（system = 跟随系统） */
  theme: AppTheme;
  /** 文件夹监控配置 */
  watcher: WatcherConfig;
  /** 桌面宠物配置 */
  pet: PetConfig;
  /** AI 能力配置（本地小模型优先，云端 API 可选） */
  ai: AiConfig;
}

const DEFAULTS: SettingsState = {
  defaultOutDir: "",
  locale: "system",
  theme: "system",
  watcher: { enabled: false, folder: "", targets: {} },
  pet: { enabled: false },
  ai: {
    mode: "auto",
    localChatModelId: "Qwen/Qwen2.5-0.5B-Instruct",
    search: { provider: "off", tavilyKey: "" },
    localServer: {
      baseUrl: "http://localhost:11434/v1",
      chatModel: "",
      embeddingModel: "",
    },
    cloud: {
      baseUrl: "",
      apiKey: "",
      embeddingModel: "text-embedding-3-small",
      chatModel: "gpt-4o-mini",
    },
  },
};

/** 文件 store 实例（首次 hydrate 时打开） */
let fileStore: Store | null = null;

/**
 * 应用设置 store（tauri-plugin-store 持久化为 JSON 文件）：
 * - defaultOutDir：默认输出目录
 * - locale：界面语言（跟随系统 / 中 / 英 / 日 / 韩）
 * - theme：界面主题（跟随系统 / 浅色 / 深色）
 * - watcher：文件夹监控（开关 / 目录 / 格式规则）
 * - ai：AI 能力（引擎模式 / 云端 API 配置）
 */
export const useSettingsStore = defineStore("settings", {
  state: (): SettingsState => ({ ...DEFAULTS }),
  actions: {
    /** 从本地文件加载设置（应用启动时调用） */
    async hydrate() {
      try {
        fileStore = await loadStore(FILE, { autoSave: true });
        const saved = (await fileStore.get<Partial<SettingsState>>("settings")) ?? {};
        this.defaultOutDir = saved.defaultOutDir ?? "";
        this.locale = saved.locale ?? "system";
        this.theme = saved.theme ?? "system";
        this.watcher = {
          enabled: saved.watcher?.enabled ?? false,
          folder: saved.watcher?.folder ?? "",
          targets: saved.watcher?.targets ?? {},
        };
        this.pet = { enabled: saved.pet?.enabled ?? false };
        this.ai = {
          mode: saved.ai?.mode ?? "auto",
          localChatModelId: saved.ai?.localChatModelId ?? "Qwen/Qwen2.5-0.5B-Instruct",
          localServer: {
            baseUrl: saved.ai?.localServer?.baseUrl ?? "http://localhost:11434/v1",
            chatModel: saved.ai?.localServer?.chatModel ?? "",
            embeddingModel: saved.ai?.localServer?.embeddingModel ?? "",
          },
          cloud: {
            baseUrl: saved.ai?.cloud?.baseUrl ?? "",
            apiKey: saved.ai?.cloud?.apiKey ?? "",
            embeddingModel: saved.ai?.cloud?.embeddingModel ?? "text-embedding-3-small",
            chatModel: saved.ai?.cloud?.chatModel ?? "gpt-4o-mini",
          },
          search: {
            provider: saved.ai?.search?.provider ?? "off",
            tavilyKey: saved.ai?.search?.tavilyKey ?? "",
          },
        };
      } catch {
        // 非 Tauri 环境（如纯浏览器预览）时静默使用默认值
      }
      // 预计算平台默认输出目录（独立 try，失败不影响设置加载）
      try {
        const dir = await invoke<string>("get_default_output_dir");
        setPlatformDefaultDir(dir);
      } catch {
        // 非 Tauri 环境，回退到源同目录
      }
    },
    /** 写回本地文件（autoSave 自动落盘） */
    save() {
      void fileStore?.set("settings", {
        defaultOutDir: this.defaultOutDir,
        locale: this.locale,
        theme: this.theme,
        watcher: this.watcher,
        pet: this.pet,
        ai: this.ai,
      });
    },
    /** 设置默认输出目录（持久化） */
    setDefaultOutDir(dir: string) {
      this.defaultOutDir = dir;
      this.save();
    },
    /** 清除默认输出目录，恢复为输出到输入文件所在目录 */
    clearDefaultOutDir() {
      this.defaultOutDir = "";
      this.save();
    },
    /** 切换界面语言 */
    setLocale(locale: AppLocale) {
      this.locale = locale;
      this.save();
    },
    /** 切换界面主题 */
    setTheme(theme: AppTheme) {
      this.theme = theme;
      this.save();
    },
    /** 更新文件夹监控配置（启用开关 / 监控目录 / 格式规则） */
    setWatcher(watcher: WatcherConfig) {
      this.watcher = watcher;
      this.save();
    },
    /** 开关桌面宠物（持久化并即时创建/关闭窗口） */
    async setPetEnabled(enabled: boolean) {
      this.pet = { enabled };
      this.save();
      try {
        const { petShow, petHide } = await import("../api");
        await (enabled ? petShow() : petHide());
      } catch {
        // 非 Tauri 环境忽略（仅保存设置）
      }
    },
    /** 更新 AI 配置（引擎模式 / 云端 API 参数） */
    setAiConfig(ai: AiConfig) {
      this.ai = ai;
      this.save();
    },
  },
});
