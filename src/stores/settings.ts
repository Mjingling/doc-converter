import { defineStore } from "pinia";
import { load as loadStore, type Store } from "@tauri-apps/plugin-store";

/** 持久化文件名（位于应用数据目录，macOS: ~/Library/Application Support/<identifier>/） */
const FILE = "settings.json";
/** 旧版 localStorage 键（一次性迁移用） */
const LEGACY_KEY = "doc-converter:settings";

export type AppLocale = "system" | "zh-CN" | "en-US" | "ja-JP" | "ko-KR";
export type AppTheme = "system" | "light" | "dark";

export type AiMode = "auto" | "local" | "cloud";

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

/** AI 能力配置：mode 引擎模式，cloud 云端 API 参数，localChatModelId 本地生成式模型 */
export interface AiConfig {
  mode: AiMode;
  cloud: CloudAiConfig;
  /** 本地生成式模型（chat）HuggingFace 模型 ID，如 Qwen/Qwen2.5-0.5B-Instruct */
  localChatModelId: string;
}

/** 文件夹监控配置：enabled 开关、folder 监控目录、targets 格式规则（扩展名 → 目标扩展名） */
export interface WatcherConfig {
  enabled: boolean;
  folder: string;
  targets: Record<string, string>;
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
  /** AI 能力配置（本地小模型优先，云端 API 可选） */
  ai: AiConfig;
}

const DEFAULTS: SettingsState = {
  defaultOutDir: "",
  locale: "system",
  theme: "system",
  watcher: { enabled: false, folder: "", targets: {} },
  ai: {
    mode: "auto",
    localChatModelId: "Qwen/Qwen2.5-0.5B-Instruct",
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
    /** 从本地文件加载设置（应用启动时调用）；首次运行自动迁移旧版 localStorage 数据 */
    async hydrate() {
      try {
        fileStore = await loadStore(FILE, { autoSave: true });
        const hasSaved = await fileStore.has("settings");
        // 旧版 localStorage 一次性迁移：文件为空且存在旧数据时导入并清除旧键
        const legacy = localStorage.getItem(LEGACY_KEY);
        if (!hasSaved && legacy) {
          try {
            const old = JSON.parse(legacy);
            if (old && typeof old === "object") {
              await fileStore.set("settings", old);
            }
          } catch {
            // 旧格式：整个值就是输出目录字符串
            await fileStore.set("settings", { defaultOutDir: legacy });
          }
          localStorage.removeItem(LEGACY_KEY);
        }
        const saved = (await fileStore.get<Partial<SettingsState>>("settings")) ?? {};
        this.defaultOutDir = saved.defaultOutDir ?? "";
        this.locale = saved.locale ?? "system";
        this.theme = saved.theme ?? "system";
        this.watcher = {
          enabled: saved.watcher?.enabled ?? false,
          folder: saved.watcher?.folder ?? "",
          targets: saved.watcher?.targets ?? {},
        };
        this.ai = {
          mode: saved.ai?.mode ?? "auto",
          localChatModelId: saved.ai?.localChatModelId ?? "Qwen/Qwen2.5-0.5B-Instruct",
          cloud: {
            baseUrl: saved.ai?.cloud?.baseUrl ?? "",
            apiKey: saved.ai?.cloud?.apiKey ?? "",
            embeddingModel: saved.ai?.cloud?.embeddingModel ?? "text-embedding-3-small",
            chatModel: saved.ai?.cloud?.chatModel ?? "gpt-4o-mini",
          },
        };
      } catch {
        // 非 Tauri 环境（如纯浏览器预览）时静默使用默认值
      }
    },
    /** 写回本地文件（autoSave 自动落盘） */
    save() {
      void fileStore?.set("settings", {
        defaultOutDir: this.defaultOutDir,
        locale: this.locale,
        theme: this.theme,
        watcher: this.watcher,
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
    /** 更新 AI 配置（引擎模式 / 云端 API 参数） */
    setAiConfig(ai: AiConfig) {
      this.ai = ai;
      this.save();
    },
  },
});
