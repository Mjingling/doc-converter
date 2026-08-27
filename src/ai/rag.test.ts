import { describe, it, expect, vi, beforeEach } from "vitest";

// mock Transformers.js：避免测试环境加载 onnxruntime 原生模块（rag 只用 local.ts 的纯函数）
vi.mock("@huggingface/transformers", () => ({
  pipeline: vi.fn(),
  env: {},
}));

// mock 云端/本地路由的 embed：按文本内容返回确定性向量，便于断言检索排序
vi.mock("./router", () => ({
  embed: vi.fn(),
}));

import { embed } from "./router";
import { buildIndex, searchTopK, buildQaMessages, RAG_BATCH, RAG_CHUNK_LEN } from "./rag";
import type { RagDocEntry } from "./rag";

const embedMock = embed as ReturnType<typeof vi.fn>;

/** 简单向量规则：含 "apple" → [1,0]；含 "banana" → [0,1]；否则 [0.5,0.5]；问题同理 */
function vec(text: string): number[] {
  if (text.includes("apple")) return [1, 0];
  if (text.includes("banana")) return [0, 1];
  return [0.5, 0.5];
}

beforeEach(() => {
  embedMock.mockReset();
  embedMock.mockImplementation(async (texts: string[]) => texts.map(vec));
});

describe("buildIndex", () => {
  it("按行分块且每批最多 RAG_BATCH 块", async () => {
    // 40 行 → 3 批（16 + 16 + 8）
    const text = Array.from({ length: 40 }, (_, i) => `line ${i}`).join("\n");
    const progress: [number, number][] = [];
    const index = await buildIndex(text, (d, t) => progress.push([d, t]));
    expect(index.chunks).toHaveLength(40);
    expect(index.vecs).toHaveLength(40);
    expect(embedMock).toHaveBeenCalledTimes(3);
    expect(embedMock.mock.calls[0][0]).toHaveLength(RAG_BATCH);
    expect(embedMock.mock.calls[2][0]).toHaveLength(8);
    expect(progress).toEqual([
      [1, 3],
      [2, 3],
      [3, 3],
    ]);
  });

  it("超过 RAG_CHUNK_LEN 的单行被切成不超过块长的片段", async () => {
    const text = "x".repeat(RAG_CHUNK_LEN * 2 + 100); // 900 字符单行 → 400+400+100
    const index = await buildIndex(text);
    expect(index.chunks).toHaveLength(3);
    for (const c of index.chunks) {
      expect(c.length).toBeLessThanOrEqual(RAG_CHUNK_LEN);
    }
    expect(index.chunks.join("")).toEqual(text);
  });

  it("空文本返回空索引且不发 embed 请求", async () => {
    const index = await buildIndex("   \n  ");
    expect(index.chunks).toHaveLength(0);
    expect(index.vecs).toHaveLength(0);
    expect(embedMock).not.toHaveBeenCalled();
  });
});

describe("searchTopK", () => {
  const mkEntry = (docId: string, docName: string, texts: string[]): RagDocEntry => ({
    docId,
    docName,
    index: { chunks: texts, vecs: texts.map(vec) },
  });

  it("按余弦相似度降序返回且跨文档检索", async () => {
    const docs = [
      mkEntry("d1", "a.pdf", ["banana recipe", "neutral note"]),
      mkEntry("d2", "b.pdf", ["apple guide", "apple pie"]),
    ];
    const hits = await searchTopK(docs, "tell me about apple", 3);
    expect(hits).toHaveLength(3);
    // 最相关的两条 apple 片段排前，且来源文档正确
    expect(hits[0].docId).toBe("d2");
    expect(hits[1].docId).toBe("d2");
    expect(hits[0].score).toBeGreaterThanOrEqual(hits[1].score);
    expect(hits[1].score).toBeGreaterThanOrEqual(hits[2].score);
  });

  it("默认取 4 条、k 可覆盖", async () => {
    const docs = [mkEntry("d1", "a.pdf", ["c1", "c2", "c3", "c4", "c5", "c6"])];
    expect(await searchTopK(docs, "q")).toHaveLength(4);
    expect(await searchTopK(docs, "q", 2)).toHaveLength(2);
  });
});

describe("buildQaMessages", () => {
  it("system 含仅依据片段与引用编号约束，user 含片段与问题", () => {
    const hits = [
      { docId: "d1", docName: "a.pdf", chunkIndex: 0, text: "片段内容一", score: 0.9 },
      { docId: "d2", docName: "b.pdf", chunkIndex: 2, text: "片段内容二", score: 0.8 },
    ];
    const msgs = buildQaMessages(hits, "问题是什么");
    expect(msgs).toHaveLength(2);
    const sys = msgs[0].content as string;
    expect(msgs[0].role).toBe("system");
    expect(sys).toContain("仅依据");
    expect(sys).toContain("片段编号");
    const user = msgs[1].content as string;
    expect(msgs[1].role).toBe("user");
    expect(user).toContain("片段内容一");
    expect(user).toContain("片段内容二");
    expect(user).toContain("a.pdf");
    expect(user).toContain("问题是什么");
    // 片段按命中顺序编号
    expect(user.indexOf("[片段 1]")).toBeLessThan(user.indexOf("[片段 2]"));
  });
});
