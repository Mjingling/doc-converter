import { describe, it, expect, vi } from "vitest";

// mock Transformers.js：避免测试环境加载 onnxruntime 原生模块（本文件只测纯函数）
vi.mock("@huggingface/transformers", () => ({
  pipeline: vi.fn(),
  env: {},
}));

import { pickRemoteHost } from "./local";

describe("pickRemoteHost 下载源选择", () => {
  it("官方可达时保持默认（海外网络）", () => {
    expect(pickRemoteHost(true, true)).toBeNull();
    expect(pickRemoteHost(true, false)).toBeNull();
  });

  it("官方被墙、镜像可达时切换镜像（国内网络）", () => {
    expect(pickRemoteHost(false, true)).toBe("https://hf-mirror.com");
  });

  it("都不可达时保持默认，由 pipeline 报原始错误", () => {
    expect(pickRemoteHost(false, false)).toBeNull();
  });
});
