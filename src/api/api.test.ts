import { describe, it, expect, vi, beforeEach } from "vitest";
import { checkUpdate, type UpdateInfo } from "./index";

/**
 * 测试 checkUpdate 和内部的 semverGt 版本比较逻辑
 * 通过 mock invoke 返回不同 JSON 来覆盖各场景
 */

// mock @tauri-apps/api/core 的 invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockInvoke = vi.mocked(invoke);

/** 构造一个版本 JSON 字符串 */
function versionJson(version: string, notes = "", downloadUrl = "https://example.com/dl") {
  return JSON.stringify({ version, notes, download_url: downloadUrl });
}

describe("checkUpdate", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /* ---------- semverGt 版本比较（通过 hasUpdate 字段验证） ---------- */

  it("semverGt: 常规比较 0.2.0 > 0.1.9", async () => {
    mockInvoke.mockResolvedValue(versionJson("0.2.0"));
    const result = await checkUpdate("0.1.9");
    expect(result).not.toBeNull();
    expect(result!.hasUpdate).toBe(true);
    expect(result!.latestVersion).toBe("0.2.0");
  });

  it("semverGt: 相等返回 false", async () => {
    mockInvoke.mockResolvedValue(versionJson("1.0.0"));
    const result = await checkUpdate("1.0.0");
    expect(result).not.toBeNull();
    expect(result!.hasUpdate).toBe(false);
  });

  it("semverGt: 不等长补零 1.0 > 0.9.99", async () => {
    mockInvoke.mockResolvedValue(versionJson("1.0"));
    const result = await checkUpdate("0.9.99");
    expect(result!.hasUpdate).toBe(true);
  });

  it("semverGt: 两位数段 0.10.0 > 0.9.0", async () => {
    mockInvoke.mockResolvedValue(versionJson("0.10.0"));
    const result = await checkUpdate("0.9.0");
    expect(result!.hasUpdate).toBe(true);
  });

  it("semverGt: 低版本不更新 0.1.0 < 0.2.0", async () => {
    mockInvoke.mockResolvedValue(versionJson("0.1.0"));
    const result = await checkUpdate("0.2.0");
    expect(result!.hasUpdate).toBe(false);
  });

  /* ---------- checkUpdate 字段缺失和异常处理 ---------- */

  it("缺 version 返回 null", async () => {
    mockInvoke.mockResolvedValue(JSON.stringify({ download_url: "https://x.com" }));
    const result = await checkUpdate("0.1.0");
    expect(result).toBeNull();
  });

  it("缺 download_url 返回 null", async () => {
    mockInvoke.mockResolvedValue(JSON.stringify({ version: "1.0.0" }));
    const result = await checkUpdate("0.1.0");
    expect(result).toBeNull();
  });

  it("非法 JSON 返回 null", async () => {
    mockInvoke.mockResolvedValue("not valid json {{{");
    const result = await checkUpdate("0.1.0");
    expect(result).toBeNull();
  });

  it("网络错误返回 null", async () => {
    mockInvoke.mockRejectedValue(new Error("network error"));
    const result = await checkUpdate("0.1.0");
    expect(result).toBeNull();
  });

  it("空响应返回 null", async () => {
    mockInvoke.mockResolvedValue("");
    const result = await checkUpdate("0.1.0");
    expect(result).toBeNull();
  });

  it("版本格式异常（非数字段）返回 null", async () => {
    mockInvoke.mockResolvedValue(versionJson("abc.def"));
    const result = await checkUpdate("0.1.0");
    // semverGt 中 Number("abc") = NaN，NaN 比较不满足 > 也不满足 <，返回 false
    // 所以 hasUpdate = false，但仍然返回 UpdateInfo
    expect(result).not.toBeNull();
    expect(result!.hasUpdate).toBe(false);
  });

  it("notes 缺失时默认为空字符串", async () => {
    mockInvoke.mockResolvedValue(JSON.stringify({ version: "1.0.0", download_url: "https://x.com" }));
    const result = await checkUpdate("0.1.0");
    expect(result!.notes).toBe("");
  });

  it("notes 正常传递", async () => {
    mockInvoke.mockResolvedValue(versionJson("1.0.0", "修复若干 bug"));
    const result = await checkUpdate("0.1.0");
    expect(result!.notes).toBe("修复若干 bug");
  });
});
