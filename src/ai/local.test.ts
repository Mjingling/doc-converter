import { describe, it, expect, vi } from "vitest";

// mock Transformers.js：避免测试环境加载 onnxruntime 原生模块（本文件只测纯函数）
vi.mock("@huggingface/transformers", () => ({
  pipeline: vi.fn(),
  env: {},
}));

import { pickRemoteHost, LocalProvider } from "./local";
import { pipeline } from "@huggingface/transformers";

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

describe("downloadEmbedModel 进度广播", () => {
  it("下载由推理侧先触发（无进度回调）时，设置页仍可订阅到进度", async () => {
    let progressCb: ((p: unknown) => void) | undefined;
    let resolveLoad!: (v: unknown) => void;
    const loadDone = new Promise((r) => {
      resolveLoad = r;
    });
    vi.mocked(pipeline).mockImplementation((..._args: unknown[]) => {
      const opts = _args[2] as { progress_callback?: (p: unknown) => void } | undefined;
      progressCb = opts?.progress_callback;
      return loadDone as never;
    });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true }));

    const provider = new LocalProvider();
    // 推理侧先触发加载（不带进度回调）
    const inferPromise = provider.embed(["hello"]).catch(() => undefined);
    // 等待在途加载真正启动（pipeline 被调用）
    for (let i = 0; i < 50 && !progressCb; i++) await new Promise((r) => setTimeout(r, 5));
    expect(progressCb).toBeTruthy();

    const events: number[] = [];
    const downloadPromise = provider.downloadEmbedModel((p) => events.push(p.percent));

    // 模拟 Transformers.js 下载事件：订阅后应即时收到广播
    progressCb!({ status: "download", file: "model.onnx", loaded: 50, total: 100 });
    progressCb!({ status: "download", file: "model.onnx", loaded: 100, total: 100 });
    resolveLoad((() => ({ tolist: () => [[0]] })) as unknown);

    await downloadPromise;
    await inferPromise;
    expect(events).toEqual([50, 100]);
    vi.unstubAllGlobals();
  });
});
