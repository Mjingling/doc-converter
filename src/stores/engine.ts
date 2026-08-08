import { defineStore } from "pinia";
import { load as loadStore, type Store } from "@tauri-apps/plugin-store";
import { getEngineStatus } from "../api";
import type { EngineMode } from "../types";

/** 持久化文件名（与 settings.ts 分开，避免键冲突） */
const FILE = "engine.json";
/** 旧版 localStorage 键（一次性迁移用） */
const LEGACY_KEY = "doc-converter:engine-mode";

/** 文件 store 实例（首次 hydrate 时打开） */
let fileStore: Store | null = null;

/**
 * 引擎模式 store（tauri-plugin-store 持久化为 JSON 文件）：
 * - builtin：内置引擎（PDF 处理，零外部依赖），默认模式
 * - libreoffice：LibreOffice 引擎（文档转换，需自行安装）
 */
export const useEngineStore = defineStore("engine", {
  state: () => ({
    /** 当前引擎模式，默认内置引擎 */
    mode: "builtin" as EngineMode,
    /** LibreOffice 是否已安装 */
    available: false,
    /** LibreOffice 可执行文件路径 */
    path: null as string | null,
  }),
  actions: {
    /** 从本地文件加载引擎模式（应用启动时调用）；首次运行自动迁移旧版 localStorage 数据 */
    async hydrate() {
      try {
        fileStore = await loadStore(FILE, { autoSave: true });
        // 旧版 localStorage 一次性迁移
        const legacy = localStorage.getItem(LEGACY_KEY);
        if (legacy && !(await fileStore.has("mode"))) {
          await fileStore.set("mode", legacy);
          localStorage.removeItem(LEGACY_KEY);
        }
        const val = await fileStore.get<string>("mode");
        this.mode = (val && (val === "builtin" || val === "libreoffice") ? val : "builtin") as EngineMode;
      } catch {
        // 非 Tauri 环境（如纯浏览器预览）时使用默认值
      }
    },
    /** 写回本地文件（autoSave 自动落盘） */
    save() {
      void fileStore?.set("mode", this.mode);
    },
    /** 重新检测 LibreOffice 引擎状态（应用启动或用户点击重新检测时调用），返回是否可用 */
    async refresh(): Promise<boolean> {
      const st = await getEngineStatus();
      this.available = st.available;
      this.path = st.path;
      // 若处于 LibreOffice 模式但引擎不可用，自动回退到内置引擎
      if (this.mode === "libreoffice" && !this.available) {
        this.mode = "builtin";
        this.save();
      }
      return this.available;
    },
    /** 切换到内置引擎 */
    useBuiltin() {
      this.mode = "builtin";
      this.save();
    },
    /** 切换到 LibreOffice 引擎；未安装时返回 false */
    useLibreOffice(): boolean {
      if (!this.available) return false;
      this.mode = "libreoffice";
      this.save();
      return true;
    },
  },
});
