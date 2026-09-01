import { describe, it, expect, beforeEach, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useTaskPoolStore, FINISHED_LINGER_MS } from "./taskPool";

/**
 * 任务池 store 测试：登记 / 进度更新 / 结束 / 驻留清理
 */

describe("taskPool store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("begin 登记运行中任务，runningTasks 正确", () => {
    const pool = useTaskPoolStore();
    pool.begin("merge", "合并 PDF");
    pool.begin("compress", "压缩 PDF");
    expect(pool.tasks.length).toBe(2);
    expect(pool.runningTasks.length).toBe(2);
    expect(pool.runningTasks[0]).toMatchObject({ panelId: "merge", label: "合并 PDF", running: true });
  });

  it("update 更新进度；未知 id 静默忽略", () => {
    const pool = useTaskPoolStore();
    const id = pool.begin("batch", "批量处理", { progress: 0 });
    pool.update(id, { progress: 50 });
    expect(pool.tasks[0].progress).toBe(50);
    expect(() => pool.update(999, { progress: 1 })).not.toThrow();
  });

  it("end 结束任务并记录成败，justFinished 命中驻留窗口", () => {
    const pool = useTaskPoolStore();
    const id = pool.begin("split", "拆分 PDF");
    pool.end(id, true);
    expect(pool.runningTasks.length).toBe(0);
    expect(pool.justFinished.length).toBe(1);
    expect(pool.justFinished[0].ok).toBe(true);
  });

  it("justFinished 排除驻留窗口外的已完成任务", () => {
    const pool = useTaskPoolStore();
    const id = pool.begin("split", "拆分 PDF");
    pool.end(id, false);
    // 时间前移超过驻留窗口
    const realNow = Date.now;
    const now = Date.now();
    Date.now = vi.fn(() => now + FINISHED_LINGER_MS + 100);
    try {
      expect(pool.justFinished.length).toBe(0);
      // sweep 同步清掉过期条目
      pool.sweep();
      expect(pool.tasks.length).toBe(0);
    } finally {
      Date.now = realNow;
    }
  });

  it("sweep 保留运行中与驻留期内的任务", () => {
    const pool = useTaskPoolStore();
    const a = pool.begin("merge", "合并");
    const b = pool.begin("split", "拆分");
    pool.end(b, true); // 刚结束，仍在驻留期
    pool.sweep();
    expect(pool.tasks.length).toBe(2); // a 运行中 + b 驻留期内
    expect(pool.tasks.find((t) => t.id === a)?.running).toBe(true);
  });
});
