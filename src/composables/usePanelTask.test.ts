import { describe, it, expect } from "vitest";
import { nextTick, ref } from "vue";
import { usePanelTask } from "./usePanelTask";

describe("usePanelTask", () => {
  it("start/done 控制 running 状态", () => {
    const task = usePanelTask();
    expect(task.running.value).toBe(false);
    task.start();
    expect(task.running.value).toBe(true);
    task.done();
    expect(task.running.value).toBe(false);
  });

  it("run 包装器正常路径自动复位", async () => {
    const task = usePanelTask();
    const result = await task.run(async () => {
      expect(task.running.value).toBe(true);
      return 42;
    });
    expect(result).toBe(42);
    expect(task.running.value).toBe(false);
  });

  it("run 包装器异常路径也会复位（异常向上抛出）", async () => {
    const task = usePanelTask();
    await expect(
      task.run(async () => {
        throw new Error("boom");
      }),
    ).rejects.toThrow("boom");
    expect(task.running.value).toBe(false);
  });

  it("不传 total 时为不确定态，progress 恒为 0", () => {
    const task = usePanelTask();
    expect(task.isIndeterminate.value).toBe(true);
    expect(task.progress.value).toBe(0);
  });

  it("批量场景：tick 累计、progress 按百分比计算、start 重置计数", () => {
    const task = usePanelTask({ total: 4 });
    expect(task.isIndeterminate.value).toBe(false);
    task.start();
    task.tick();
    task.tick();
    expect(task.progress.value).toBe(50);
    task.start(); // 重新开始应清零
    expect(task.progress.value).toBe(0);
    task.tick();
    expect(task.progress.value).toBe(25);
  });

  it("total 支持 getter 动态跟随文件列表", async () => {
    const files = ref<string[]>(["a", "b", "c"]);
    const task = usePanelTask({ total: () => files.value.length });
    task.start();
    task.tick();
    expect(task.progress.value).toBe(33);
    files.value.push("d");
    await nextTick();
    expect(task.progress.value).toBe(25);
  });

  it("进度封顶 100%（tick 超过 total 不越界）", () => {
    const task = usePanelTask({ total: 2 });
    task.start();
    task.tick();
    task.tick();
    task.tick();
    expect(task.progress.value).toBe(100);
  });
});
