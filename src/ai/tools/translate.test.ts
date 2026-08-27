import { describe, it, expect } from "vitest";
import { chunkText, langCode, stripFences } from "./translate-utils";

describe("chunkText", () => {
  it("短文本不切分", () => {
    expect(chunkText("你好")).toEqual(["你好"]);
  });

  it("按段落贪心累计，不超限合并", () => {
    const paras = ["a".repeat(1000), "b".repeat(1000), "c".repeat(1000)];
    // 3 段共约 3000 字符 > 2500 → 前两段一块 + 第三段一块
    const chunks = chunkText(paras.join("\n\n"), 2500);
    expect(chunks.length).toBe(2);
    expect(chunks[0]).toContain("aaa");
    expect(chunks[0]).toContain("bbb");
    expect(chunks[1]).toContain("ccc");
  });

  it("单段超 2 倍上限时按句子切分", () => {
    const sentences = Array.from({ length: 10 }, (_, i) => `第${i}句话。`);
    const longPara = sentences.join(""); // 每句约 5 字符，共约 50+ 字符但远小于 limit
    // 构造真正超长的：500 句 × 10 字符 = 5000 字符 > 2×limit(200)
    const big = Array.from({ length: 500 }, (_, i) => `第${i}条内容来了。`).join("");
    expect(big.length).toBeGreaterThan(400);
    const chunks = chunkText(big, 200);
    expect(chunks.length).toBeGreaterThan(1);
    // 切分不丢内容（拼接后与原文等长）
    expect(chunks.join("")).toBe(big);
    expect(longPara).toBeTruthy();
  });

  it("切分不丢失内容（普通段落场景）", () => {
    const text = Array.from({ length: 30 }, (_, i) => `段落${i}：${"x".repeat(200)}`).join("\n\n");
    const chunks = chunkText(text, 1000);
    expect(chunks.join("\n\n")).toBe(text);
  });

  it("超长单句硬切不断开代理对", () => {
    // 表情符号（代理对）组成的超长单句
    const emoji = "😀".repeat(600); // 1200 UTF-16 码元
    const chunks = chunkText(emoji, 500);
    expect(chunks.join("")).toBe(emoji);
    for (const c of chunks) {
      // 每块长度必须是偶数（代理对完整）
      expect(c.length % 2).toBe(0);
    }
  });
});

describe("langCode", () => {
  it("常见语言映射缩写", () => {
    expect(langCode("英语")).toBe("en");
    expect(langCode("English")).toBe("en");
    expect(langCode("日语")).toBe("ja");
    expect(langCode("韩文")).toBe("ko");
    expect(langCode("简体中文")).toBe("zh");
  });

  it("ISO 码直接透传", () => {
    expect(langCode("fr")).toBe("fr");
    expect(langCode("PT")).toBe("pt");
    expect(langCode("zh-TW")).toBe("zh-tw");
  });

  it("未知语言名 sanitize 后使用", () => {
    expect(langCode("Swahili")).toBe("swahili");
    expect(langCode("印尼语")).toBe("印尼语");
    // 全符号输入回退默认
    expect(langCode("///")).toBe("translated");
  });
});

describe("stripFences", () => {
  it("去除首尾代码围栏", () => {
    expect(stripFences("```\n译文内容\n```")).toBe("译文内容");
    expect(stripFences("```markdown\n# 标题\n```")).toBe("# 标题");
  });

  it("无围栏原样返回", () => {
    expect(stripFences("普通译文")).toBe("普通译文");
  });

  it("只有首围栏不去除（避免误删半截）", () => {
    expect(stripFences("```\nabc")).toBe("```\nabc");
  });
});
