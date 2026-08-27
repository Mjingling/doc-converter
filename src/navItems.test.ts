import { describe, it, expect, vi } from "vitest";

// navItems.ts 依赖图标库，测试中用占位组件替代
vi.mock("@vicons/ionicons5", () =>
  new Proxy({}, { get: (_t, name) => ({ name }), has: () => true })
);

import { fuzzyMatch, filterNavItems, flattenNavItems, navGroups } from "./navItems";

describe("fuzzyMatch", () => {
  it("空查询匹配任意文本", () => {
    expect(fuzzyMatch("", "anything")).toBe(true);
    expect(fuzzyMatch("   ", "anything")).toBe(true);
  });

  it("忽略大小写的连续子串匹配", () => {
    expect(fuzzyMatch("merge", "PDF 合并 Merge")).toBe(true);
    expect(fuzzyMatch("MERGE", "merge")).toBe(true);
    expect(fuzzyMatch("xyz", "merge")).toBe(false);
  });
});

describe("filterNavItems", () => {
  it("导航分组数据完整且 id 不重复", () => {
    const items = flattenNavItems();
    expect(items.length).toBeGreaterThanOrEqual(30);
    expect(new Set(items.map((i) => i.id)).size).toBe(items.length);
    // 每个分组都有 title 与 engine 字段
    for (const g of navGroups) {
      expect(["builtin", "libreoffice", "none"]).toContain(g.engine);
      expect(g.items.length).toBeGreaterThan(0);
    }
  });

  it("空查询返回全部导航项", () => {
    expect(filterNavItems("", (k) => k)).toEqual(flattenNavItems());
  });

  it("按 i18n 名称与 id 过滤", () => {
    // labelOf 恒等 → 匹配 label key（如 nav.merge 含 merge）
    const byKey = filterNavItems("merge", (k) => k);
    expect(byKey.some((i) => i.id === "merge")).toBe(true);
    // 翻译后的中文名也能匹配
    const byLabel = filterNavItems("签名", (k) => (k === "nav.signature" ? "电子签名" : k));
    expect(byLabel.some((i) => i.id === "signature")).toBe(true);
    // id 直接匹配
    const byId = filterNavItems("pdfRender", (k) => k);
    expect(byId.some((i) => i.id === "pdfRender")).toBe(true);
  });

  it("无匹配时返回空数组", () => {
    expect(filterNavItems("不存在的功能xyz", (k) => k)).toEqual([]);
  });
});
