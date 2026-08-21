import { describe, it, expect, beforeEach } from "vitest";
import {
  extOf,
  dirOf,
  setPlatformDefaultDir,
  defaultOutDir,
  defaultOutputPath,
} from "./file";

describe("extOf", () => {
  it("提取普通扩展名", () => {
    expect(extOf("test.pdf")).toBe("pdf");
    expect(extOf("a.b.docx")).toBe("docx");
  });

  it("无扩展名返回空字符串", () => {
    expect(extOf("README")).toBe("");
    expect(extOf("/path/to/noext")).toBe("");
  });

  it("以点开头的文件名不误判扩展名", () => {
    expect(extOf(".gitignore")).toBe(""); // lastIndexOf 返回 0，i > 0 为 false
    expect(extOf(".env")).toBe("");
  });

  it("兼容 Windows 反斜杠路径", () => {
    expect(extOf("C:\\Users\\test\\file.pdf")).toBe("pdf");
  });
});

describe("dirOf", () => {
  it("Unix 路径提取目录", () => {
    expect(dirOf("/home/user/docs/test.pdf")).toBe("/home/user/docs");
  });

  it("Windows 路径提取目录", () => {
    expect(dirOf("C:\\Users\\test\\file.pdf")).toBe("C:\\Users\\test");
  });

  it("无分隔符时返回原路径", () => {
    expect(dirOf("file.pdf")).toBe("file.pdf");
  });

  it("根路径", () => {
    expect(dirOf("/file.pdf")).toBe("");
  });
});

describe("defaultOutDir（三级回退）", () => {
  beforeEach(() => {
    setPlatformDefaultDir(""); // 每个用例前重置平台默认
  });

  it("第一优先级：用户设置非空时直接返回", () => {
    setPlatformDefaultDir("/platform/default");
    expect(defaultOutDir("/src/file.pdf", "/user/setting")).toBe("/user/setting");
  });

  it("第二优先级：无用户设置时用平台默认", () => {
    setPlatformDefaultDir("/platform/default");
    expect(defaultOutDir("/src/file.pdf")).toBe("/platform/default");
  });

  it("第三优先级：无用户设置且无平台默认时用源文件目录", () => {
    expect(defaultOutDir("/src/dir/file.pdf")).toBe("/src/dir");
  });

  it("空字符串 settingsDefault 不视为已设置", () => {
    setPlatformDefaultDir("/platform");
    expect(defaultOutDir("/src/file.pdf", "")).toBe("/platform");
  });
});

describe("defaultOutputPath", () => {
  beforeEach(() => {
    setPlatformDefaultDir("");
  });

  it("默认生成 .pdf 扩展名", () => {
    const out = defaultOutputPath("/src/doc.pdf", "_merged");
    expect(out).toBe("/src/doc_merged.pdf");
  });

  it("使用用户设置目录", () => {
    const out = defaultOutputPath("/src/doc.pdf", "_merged", "/output");
    expect(out).toBe("/output/doc_merged.pdf");
  });

  it("使用平台默认目录", () => {
    setPlatformDefaultDir("/platform/out");
    const out = defaultOutputPath("/src/doc.pdf", "_compressed");
    expect(out).toBe("/platform/out/doc_compressed.pdf");
  });

  it("回退到源同目录", () => {
    const out = defaultOutputPath("/src/dir/file.pdf", "_rotated");
    expect(out).toBe("/src/dir/file_rotated.pdf");
  });

  it("自定义扩展名", () => {
    const out = defaultOutputPath("/src/file.pdf", "_data", undefined, ".csv");
    expect(out).toBe("/src/file_data.csv");
  });

  it("Windows 路径分隔符", () => {
    const out = defaultOutputPath("C:\\docs\\file.pdf", "_split");
    expect(out).toBe("C:\\docs/file_split.pdf");
  });

  it("无扩展名的源文件", () => {
    const out = defaultOutputPath("/src/README", "_x");
    expect(out).toBe("/src/README_x.pdf");
  });
});
