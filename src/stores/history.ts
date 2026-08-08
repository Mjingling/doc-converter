import { defineStore } from "pinia";
import { load as loadStore, type Store } from "@tauri-apps/plugin-store";

/** 持久化文件名（位于应用数据目录，macOS: ~/Library/Application Support/<identifier>/） */
const FILE = "history.json";
/** 历史记录上限，超出后丢弃最旧条目 */
const MAX_ITEMS = 200;

/** 单条转换历史 */
export interface HistoryItem {
  id: string;
  /** 操作类型：merge / split / compress / watermark / rotate / encrypt / decrypt / images2pdf / convert */
  kind: string;
  /** 展示名称（结果文件名或摘要） */
  name: string;
  /** 输入路径 */
  inputs: string[];
  /** 输出路径 */
  outputs: string[];
  /** 时间戳（ms） */
  time: number;
  /** 是否成功 */
  ok: boolean;
}

/** 文件 store 实例（首次 hydrate 时打开） */
let fileStore: Store | null = null;

/**
 * 操作历史 store（tauri-plugin-store 持久化为 JSON 文件）：
 * - 记录合并/拆分/压缩/水印/旋转/加解密/图片转 PDF/文档转换 的操作与产物路径
 * - 上限 200 条，超出自动丢弃最旧条目
 */
export const useHistoryStore = defineStore("history", {
  state: () => ({
    items: [] as HistoryItem[],
    loaded: false,
  }),
  actions: {
    /** 从本地文件加载历史（应用启动时调用） */
    async hydrate() {
      try {
        fileStore = await loadStore(FILE, { autoSave: true });
        const saved = await fileStore.get<HistoryItem[]>("items");
        this.items = Array.isArray(saved) ? saved : [];
      } catch {
        // 非 Tauri 环境（如纯浏览器预览）时静默使用空列表
      } finally {
        this.loaded = true;
      }
    },
    /** 追加一条历史记录（新记录在前，超出上限截断） */
    async add(entry: Omit<HistoryItem, "id" | "time">) {
      const item: HistoryItem = {
        ...entry,
        id: `h-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        time: Date.now(),
      };
      this.items.unshift(item);
      if (this.items.length > MAX_ITEMS) this.items.length = MAX_ITEMS;
      try {
        await fileStore?.set("items", this.items);
      } catch {
        // 持久化失败不影响本次会话使用
      }
    },
    /** 删除单条历史 */
    async remove(id: string) {
      this.items = this.items.filter((i) => i.id !== id);
      try {
        await fileStore?.set("items", this.items);
      } catch {
        // 忽略
      }
    },
    /** 清空全部历史 */
    async clear() {
      this.items = [];
      try {
        await fileStore?.set("items", this.items);
      } catch {
        // 忽略
      }
    },
  },
});
