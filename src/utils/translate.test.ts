import { describe, it, expect } from "vitest";
import { splitForTranslate, TRANSLATE_CHUNK_LEN } from "./translate";

describe("splitForTranslate", () => {
  it("短段落聚合为单块", () => {
    const chunks = splitForTranslate("para one\npara two\n\npara three");
    expect(chunks).toEqual(["para one\npara two\npara three"]);
  });

  it("聚合不超过 maxLen，超出后换块", () => {
    const text = `${"a".repeat(700)}\n${"b".repeat(700)}\n${"c".repeat(700)}`;
    const chunks = splitForTranslate(text, 1200);
    expect(chunks).toHaveLength(3);
    for (const c of chunks) expect(c.length).toBeLessThanOrEqual(TRANSLATE_CHUNK_LEN);
    // 内容无丢失
    expect(chunks.join("\n").replace(/\n+/g, "")).toEqual("a".repeat(700) + "b".repeat(700) + "c".repeat(700));
  });

  it("超长单段被硬切为不超过 maxLen 的块", () => {
    const text = "x".repeat(2500);
    const chunks = splitForTranslate(text, 1000);
    expect(chunks).toEqual(["x".repeat(1000), "x".repeat(1000), "x".repeat(500)]);
  });

  it("空文本返回空数组", () => {
    expect(splitForTranslate("  \n \n")).toEqual([]);
  });
});
