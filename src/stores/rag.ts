import { defineStore } from "pinia";
import type { RagHit, RagIndex } from "../ai/rag";

/** 已加入问答的文档：索引存内存，切走面板不丢失（不持久化，重启后重建） */
export interface RagDoc {
  id: string;
  /** 文件名（展示用） */
  name: string;
  /** 源文件绝对路径 */
  path: string;
  status: "indexing" | "ready" | "failed";
  /** 索引进度（embed 批次） */
  batchDone: number;
  batchTotal: number;
  error?: string;
  index?: RagIndex;
}

/** 问答消息流（assistant 消息携带引用片段） */
export interface RagMsg {
  id: number;
  role: "user" | "assistant";
  content: string;
  hits?: RagHit[];
}

let seq = 0;

export const useRagStore = defineStore("rag", {
  state: () => ({
    docs: [] as RagDoc[],
    msgs: [] as RagMsg[],
  }),
  getters: {
    /** 索引就绪、可参与检索的文档 */
    readyDocs: (s): RagDoc[] => s.docs.filter((d) => d.status === "ready" && d.index),
  },
  actions: {
    addDoc(path: string): RagDoc {
      const doc: RagDoc = {
        id: `doc_${++seq}_${Date.now()}`,
        name: path.split(/[/\\]/).pop() || path,
        path,
        status: "indexing",
        batchDone: 0,
        batchTotal: 1,
      };
      this.docs.push(doc);
      return doc;
    },
    removeDoc(id: string) {
      this.docs = this.docs.filter((d) => d.id !== id);
    },
    addMsg(msg: Omit<RagMsg, "id">): RagMsg {
      const m: RagMsg = { ...msg, id: ++seq };
      this.msgs.push(m);
      return m;
    },
    clearMsgs() {
      this.msgs = [];
    },
  },
});
