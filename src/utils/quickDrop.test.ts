import { describe, it, expect } from "vitest";
import { parseQuickDropActions } from "./quickDrop";

describe("parseQuickDropActions", () => {
  it("PDF 提供转换 / 压缩 / AI 摘要", () => {
    expect(parseQuickDropActions("/a/report.pdf")).toEqual(["convert", "compress", "aiSummary"]);
  });

  it("Office 文档提供转换；docx 可 AI 摘要，xlsx 仅转换", () => {
    expect(parseQuickDropActions("/a/合同.docx")).toEqual(["convert", "aiSummary"]);
    expect(parseQuickDropActions("/a/data.XLSX")).toEqual(["convert"]);
  });

  it("图片仅提供合成 PDF", () => {
    expect(parseQuickDropActions("/a/photo.png")).toEqual(["images2pdf"]);
    expect(parseQuickDropActions("/a/photo.jpeg")).toEqual(["images2pdf"]);
  });

  it("纯文本提供转换与 AI 摘要", () => {
    expect(parseQuickDropActions("/a/notes.md")).toEqual(["convert", "aiSummary"]);
    expect(parseQuickDropActions("/a/plain.txt")).toEqual(["convert", "aiSummary"]);
  });

  it("未知扩展名返回空列表", () => {
    expect(parseQuickDropActions("/a/video.mp4")).toEqual([]);
    expect(parseQuickDropActions("/a/noext")).toEqual([]);
  });
});
