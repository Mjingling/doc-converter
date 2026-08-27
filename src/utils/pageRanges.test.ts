import { describe, it, expect } from "vitest";
import { parsePageRanges, PageRangeError } from "./pageRanges";

describe("parsePageRanges", () => {
  it("解析单页、区间与混合输入", () => {
    expect(parsePageRanges("1", 10)).toEqual([1]);
    expect(parsePageRanges("1-3", 10)).toEqual([1, 2, 3]);
    expect(parsePageRanges("1-3, 5, 8-9", 10)).toEqual([1, 2, 3, 5, 8, 9]);
  });

  it("去重并保持首次出现顺序", () => {
    expect(parsePageRanges("2,2-3,1", 10)).toEqual([2, 3, 1]);
  });

  it("越界报错", () => {
    expect(() => parsePageRanges("11", 10)).toThrow(PageRangeError);
    expect(() => parsePageRanges("5-11", 10)).toThrow(PageRangeError);
  });

  it("非法格式报错", () => {
    expect(() => parsePageRanges("", 10)).toThrow(PageRangeError);
    expect(() => parsePageRanges("abc", 10)).toThrow(PageRangeError);
    expect(() => parsePageRanges("3-1", 10)).toThrow(PageRangeError);
    expect(() => parsePageRanges("0", 10)).toThrow(PageRangeError);
  });
});
