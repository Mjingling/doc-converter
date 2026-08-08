import { defineStore } from "pinia";
import type { ConvertTask } from "../types";

let seq = 0;

export const useQueueStore = defineStore("queue", {
  state: () => ({
    tasks: [] as ConvertTask[],
  }),
  actions: {
    addTask(inputPath: string, fileName: string, targetExt: string): string {
      const id = `task-${++seq}-${Date.now()}`;
      this.tasks.push({ id, inputPath, fileName, targetExt, status: "pending" });
      return id;
    },
    setStatus(id: string, status: ConvertTask["status"], extra?: Partial<ConvertTask>) {
      const t = this.tasks.find((t) => t.id === id);
      if (t) Object.assign(t, { status }, extra);
    },
    removeTask(id: string) {
      this.tasks = this.tasks.filter((t) => t.id !== id);
    },
    clearDone() {
      this.tasks = this.tasks.filter((t) => t.status === "converting" || t.status === "pending");
    },
  },
});
