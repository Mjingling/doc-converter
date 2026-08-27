/**
 * RAG（检索增强生成）核心逻辑：
 * - buildIndex：文档文本 → 分块 → 批量向量化
 * - searchTopK：问题向量与全量块做余弦相似度，取 top-k
 * - buildQaMessages：把命中片段拼入提示词，约束模型仅依据片段作答
 */
import { chunkText, cosine } from "./local";
import { embed } from "./router";
import type { ChatMessage } from "./types";

/** 单块最大字符数（按行分块，超长行截断） */
export const RAG_CHUNK_LEN = 400;
/** 每批向量化的块数（控制单次 embed 请求体积） */
export const RAG_BATCH = 16;
/** 默认召回片段数 */
export const RAG_TOP_K = 4;

/** 单个文档的向量索引 */
export interface RagIndex {
  chunks: string[];
  vecs: number[][];
}

/** 参与检索的文档（索引 + 元信息） */
export interface RagDocEntry {
  docId: string;
  docName: string;
  index: RagIndex;
}

/** 检索命中：片段 + 来源 + 分数 */
export interface RagHit {
  docId: string;
  docName: string;
  chunkIndex: number;
  text: string;
  score: number;
}

/**
 * 构建文档索引：chunkText 分块 → 每 RAG_BATCH 块一批调 embed
 * onProgress(已完成批次, 总批次) 用于面板进度展示
 */
export async function buildIndex(
  text: string,
  onProgress?: (doneBatches: number, totalBatches: number) => void
): Promise<RagIndex> {
  const chunks = chunkText(text, RAG_CHUNK_LEN);
  const totalBatches = Math.max(1, Math.ceil(chunks.length / RAG_BATCH));
  const vecs: number[][] = [];
  let batchNo = 0;
  for (let i = 0; i < chunks.length; i += RAG_BATCH) {
    const batch = chunks.slice(i, i + RAG_BATCH);
    const v = await embed(batch);
    vecs.push(...v);
    batchNo++;
    onProgress?.(batchNo, totalBatches);
  }
  return { chunks, vecs };
}

/** 问题 embed 后与所有文档的全部块做余弦打分，按分数降序取前 k 个 */
export async function searchTopK(docs: RagDocEntry[], question: string, k = RAG_TOP_K): Promise<RagHit[]> {
  const [qv] = await embed([question]);
  const hits: RagHit[] = [];
  for (const d of docs) {
    const { chunks, vecs } = d.index;
    for (let i = 0; i < chunks.length; i++) {
      hits.push({
        docId: d.docId,
        docName: d.docName,
        chunkIndex: i,
        text: chunks[i],
        score: cosine(qv, vecs[i]),
      });
    }
  }
  hits.sort((a, b) => b.score - a.score);
  return hits.slice(0, k);
}

/** 组装问答消息：system 约束「仅依据片段作答」+ 带编号的片段列表 + 用户问题 */
export function buildQaMessages(hits: RagHit[], question: string): ChatMessage[] {
  const ctx = hits
    .map((h, i) => `[片段 ${i + 1}]（来源：${h.docName}）\n${h.text}`)
    .join("\n\n");
  return [
    {
      role: "system",
      content:
        "你是文档问答助手。请仅依据以下文档片段回答用户问题；如果片段中没有相关信息，请明确说明无法从给定文档中找到答案，不要编造。回答中请引用片段编号（如「片段 1」）。",
    },
    { role: "user", content: `文档片段：\n${ctx}\n\n问题：${question}` },
  ];
}
