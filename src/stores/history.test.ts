import { describe, it, expect, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useHistoryStore } from "./history";

/**
 * history store 状态逻辑测试（不 hydrate 时 fileStore 为 null，
 * add/remove/clear 中的 fileStore?.set 静默跳过，仅测试内存状态）
 */
describe("useHistoryStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  function makeEntry(kind = "merge", name = "test.pdf") {
    return { kind, name, inputs: ["/a.pdf"], outputs: ["/out.pdf"], ok: true };
  }

  it("add 新记录插入头部（unshift 语义）", async () => {
    const store = useHistoryStore();
    await store.add(makeEntry("merge", "first.pdf"));
    await store.add(makeEntry("split", "second.pdf"));
    expect(store.items.length).toBe(2);
    expect(store.items[0].name).toBe("second.pdf");
    expect(store.items[1].name).toBe("first.pdf");
  });

  it("add 超 200 条自动截断到 MAX_ITEMS", async () => {
    const store = useHistoryStore();
    for (let i = 0; i < 201; i++) {
      await store.add(makeEntry("merge", `file-${i}.pdf`));
    }
    expect(store.items.length).toBe(200);
    // 最新的在前面
    expect(store.items[0].name).toBe("file-200.pdf");
  });

  it("add 恰好 200 条不截断", async () => {
    const store = useHistoryStore();
    for (let i = 0; i < 200; i++) {
      await store.add(makeEntry());
    }
    expect(store.items.length).toBe(200);
  });

  it("remove 按 id 精确删除", async () => {
    const store = useHistoryStore();
    await store.add(makeEntry("merge", "keep.pdf"));
    await store.add(makeEntry("split", "remove.pdf"));
    const targetId = store.items.find((i) => i.name === "remove.pdf")!.id;
    await store.remove(targetId);
    expect(store.items.length).toBe(1);
    expect(store.items[0].name).toBe("keep.pdf");
  });

  it("remove 不存在的 id 静默不报错", async () => {
    const store = useHistoryStore();
    await store.add(makeEntry());
    await expect(store.remove("non-existent-id")).resolves.not.toThrow();
    expect(store.items.length).toBe(1);
  });

  it("clear 清空列表", async () => {
    const store = useHistoryStore();
    await store.add(makeEntry());
    await store.add(makeEntry());
    await store.clear();
    expect(store.items.length).toBe(0);
  });

  it("add 生成唯一 id（连续 add 不重复）", async () => {
    const store = useHistoryStore();
    for (let i = 0; i < 10; i++) {
      await store.add(makeEntry());
    }
    const ids = store.items.map((i) => i.id);
    expect(new Set(ids).size).toBe(10);
  });

  it("add 自动填充 id 和 time 字段", async () => {
    const store = useHistoryStore();
    const before = Date.now();
    await store.add(makeEntry());
    const after = Date.now();
    const item = store.items[0];
    expect(item.id).toMatch(/^h-\d+-/);
    expect(item.time).toBeGreaterThanOrEqual(before);
    expect(item.time).toBeLessThanOrEqual(after);
  });
});
