import { pipeline, env, type FeatureExtractionPipeline } from "@huggingface/transformers";
import type { AiProvider, ChatMessage } from "./types";

/**
 * 本地 embedding 模型：bge-small-zh-v1.5（~100MB，中文语义匹配，向量维度 512）
 * 模型文件首次使用时从 HuggingFace 下载，之后由浏览器缓存（Cache API）离线可用
 */
const EMBED_MODEL = "Xenova/bge-small-zh-v1.5";
/** 语义相似度阈值：> REWRITE_SAME 视为相同，> REWRITE_MIN 视为改写，否则视为新增 */
export const SIM_SAME = 0.86;
export const SIM_REWRITE = 0.65;

/** Transformers.js 浏览器缓存名（v4 默认 env.cacheKey） */
const CACHE_NAME = "transformers-cache";

/** 本地 chat 模型状态 */
export type ChatModelState = "unavailable" | "downloading" | "ready";

/** 下载进度回调事件（透传 Transformers.js progress_callback；embedding 与 chat 模型共用） */
export interface ChatModelProgress {
  /** 当前下载的文件名，如 model.onnx */
  file: string;
  /** 当前文件下载进度 0-100 */
  percent: number;
  /** 已下载 / 总字节数 */
  loaded: number;
  total: number;
}

/** Transformers.js progress_callback 原始事件 → 统一进度结构（非 download 事件返回 null） */
function toProgress(p: any): ChatModelProgress | null {
  if (p?.status === "download" && p.file) {
    const total = Number(p.total) || 0;
    return {
      file: String(p.file),
      loaded: Number(p.loaded) || 0,
      total,
      percent: total > 0 ? Math.min(100, Math.round((Number(p.loaded) / total) * 100)) : 0,
    };
  }
  return null;
}

/** 模型文件是否已缓存（Cache API 命中 config.json 视为已下载） */
async function isModelCached(repoId: string): Promise<boolean> {
  try {
    const cache = await caches.open(CACHE_NAME);
    const keys = await cache.keys();
    return keys.some((k) => k.url.includes(`/${repoId}/`));
  } catch {
    return false;
  }
}

/** 删除指定模型的全部缓存条目（不影响其他模型） */
async function deleteModelCache(repoId: string): Promise<number> {
  const cache = await caches.open(CACHE_NAME);
  const keys = await cache.keys();
  const targets = keys.filter((k) => k.url.includes(`/${repoId}/`));
  await Promise.all(targets.map((k) => cache.delete(k)));
  return targets.length;
}

/** 统计指定模型缓存总大小（字节；优先 content-length 头，缺失时跳过） */
async function modelCacheSize(repoId: string): Promise<number> {
  const cache = await caches.open(CACHE_NAME);
  const keys = (await cache.keys()).filter((k) => k.url.includes(`/${repoId}/`));
  let total = 0;
  for (const key of keys) {
    const resp = await cache.match(key);
    if (!resp) continue;
    const len = Number(resp.headers.get("content-length") || 0);
    if (Number.isFinite(len) && len > 0) total += len;
  }
  return total;
}

/** 字节数 → 人类可读（MB 级精度 1 位小数） */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 MB";
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/* ---------- 模型下载源：官方源被墙时自动回退国内镜像（hf-mirror.com） ---------- */

const HF_OFFICIAL = "https://huggingface.co";
const HF_MIRROR = "https://hf-mirror.com";

/** 根据探测结果选择下载源：官方可达保持默认；否则镜像可达切镜像；都不可达返回 null（保持默认，由 pipeline 报原始错误） */
export function pickRemoteHost(officialOk: boolean, mirrorOk: boolean): string | null {
  if (officialOk || !mirrorOk) return null;
  return HF_MIRROR;
}

/** 单个 host 连通性探测：任何 HTTP 响应（含 404/307）都算可达，被墙时表现为超时/连接错误。
 *  探测 URL 必须用 resolve 路径（带 CORS 回显头）而非根路径（无 CORS 头，fetch 会被误判拦截） */
async function probeHost(host: string, timeoutMs: number): Promise<boolean> {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    await fetch(`${host}/${EMBED_MODEL}/resolve/main/config.json`, {
      method: "HEAD",
      signal: ctrl.signal,
    });
    return true;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

/** 远程下载源探测（进程内只做一次）：官方/镜像并行探测，避免国内用户串行等待双重超时 */
let remoteHostReady: Promise<void> | null = null;

function ensureRemoteHost(): Promise<void> {
  remoteHostReady ??= (async () => {
    const [officialOk, mirrorOk] = await Promise.all([
      probeHost(HF_OFFICIAL, 6000),
      probeHost(HF_MIRROR, 6000),
    ]);
    const host = pickRemoteHost(officialOk, mirrorOk);
    if (host) env.remoteHost = host;
  })();
  return remoteHostReady;
}

/** 本地推理提供方：WebView 内 WASM 运行 Transformers.js，无外部 API 依赖 */
export class LocalProvider implements AiProvider {
  readonly name = "local";
  private extractor: FeatureExtractionPipeline | null = null;
  private loading: Promise<FeatureExtractionPipeline> | null = null;

  /** 生成式模型 pipeline（onnx-community/Qwen2.5-0.5B-Instruct，WASM + q8 量化；需带 ONNX 权重的仓库） */
  private chatModelId = "onnx-community/Qwen2.5-0.5B-Instruct";
  private generator: any = null;
  private chatLoading: Promise<any> | null = null;
  private chatState: ChatModelState = "unavailable";

  /** 设置本地 chat 模型 ID（设置页变更后调用） */
  updateChatModelId(id: string) {
    if (id.trim() && id !== this.chatModelId) {
      this.chatModelId = id.trim();
      // 模型变更后旧 pipeline 失效，需要重新加载
      this.generator = null;
      this.chatLoading = null;
      this.chatState = "unavailable";
    }
  }

  /** 懒加载 pipeline；并发调用时共享同一个加载 Promise；失败时清空 loading 以允许重试 */
  private async getExtractor(onProgress?: (p: ChatModelProgress) => void): Promise<FeatureExtractionPipeline> {
    if (!this.extractor) {
      if (!this.loading) {
        this.loading = (async () => {
          // 允许模型/wasm 从远程加载（PoC 阶段；后续可切换为本地模型目录）
          env.allowRemoteModels = true;
          env.allowLocalModels = false;
          await ensureRemoteHost(); // 官方源被墙时切换国内镜像
          return pipeline("feature-extraction", EMBED_MODEL, {
            progress_callback: onProgress
              ? (p: any) => {
                  const ev = toProgress(p);
                  if (ev) onProgress(ev);
                }
              : undefined,
          });
        })();
        this.loading.catch(() => {
          this.loading = null;
        });
      }
      this.extractor = await this.loading;
    }
    return this.extractor;
  }

  async status(): Promise<"ready" | "loading" | "unavailable"> {
    if (this.extractor) return "ready";
    return this.loading ? "loading" : "unavailable";
  }

  /* ---------- 本地 embedding 模型管理（设置页展示用） ---------- */

  /** embedding 模型就绪状态：ready（已缓存或已加载）/ downloading（下载中）/ unavailable（未下载） */
  async embedStatus(): Promise<ChatModelState> {
    if (this.extractor) return "ready";
    if (this.loading) return "downloading";
    // 未加载过：探测缓存判断是否已下载（已下载但未加载也算 ready，首次推理时懒加载）
    return (await isModelCached(EMBED_MODEL)) ? "ready" : "unavailable";
  }

  /** 显式下载 embedding 模型（带进度回调）；已加载/下载中直接返回 */
  async downloadEmbedModel(onProgress: (p: ChatModelProgress) => void): Promise<void> {
    if (this.extractor || this.loading) return;
    await this.getExtractor(onProgress);
  }

  /** 删除 embedding 模型缓存（已加载的 pipeline 仍在内存，重启后生效） */
  async deleteEmbedModel(): Promise<number> {
    const n = await deleteModelCache(EMBED_MODEL);
    this.extractor = null;
    this.loading = null;
    return n;
  }

  /** embedding 模型缓存大小（字节） */
  async embedModelSize(): Promise<number> {
    return modelCacheSize(EMBED_MODEL);
  }

  async embed(texts: string[]): Promise<number[][]> {
    if (texts.length === 0) return [];
    const extractor = await this.getExtractor();
    const out = await extractor(texts, { pooling: "mean", normalize: true });
    return out.tolist() as number[][];
  }

  /* ---------- 本地生成式模型（chat） ---------- */

  /** 生成式模型就绪状态：ready（可推理）/ downloading（下载中）/ unavailable（未下载或加载失败） */
  async chatStatus(): Promise<ChatModelState> {
    if (this.generator) return "ready";
    if (this.chatState === "downloading") return "downloading";
    if (this.chatLoading) return "downloading";
    // 未加载过：探测缓存判断是否已下载（已下载但未加载也算 ready，首次推理时懒加载）
    return (await isModelCached(this.chatModelId)) ? "ready" : "unavailable";
  }

  /** 显式下载 chat 模型（带进度回调）；已缓存时秒回；下载期间模型 ID 变更则丢弃结果 */
  async downloadChatModel(onProgress: (p: ChatModelProgress) => void): Promise<void> {
    if (this.generator || this.chatLoading) return;
    this.chatState = "downloading";
    const id = this.chatModelId;
    try {
      await this.loadChatPipeline(onProgress);
      // 下载期间模型 ID 被切换，丢弃加载结果（旧模型文件虽已缓存但不被引用）
      if (this.chatModelId !== id) {
        this.generator = null;
        this.chatLoading = null;
        this.chatState = "unavailable";
        return;
      }
      this.chatState = "ready";
    } catch (e) {
      this.chatState = "unavailable";
      throw e;
    }
  }

  /** 删除 chat 模型缓存（已加载的 pipeline 仍在内存，重启后生效） */
  async deleteChatModel(): Promise<number> {
    const n = await deleteModelCache(this.chatModelId);
    this.generator = null;
    this.chatLoading = null;
    this.chatState = "unavailable";
    return n;
  }

  /** chat 模型缓存大小（字节） */
  async chatModelSize(): Promise<number> {
    return modelCacheSize(this.chatModelId);
  }

  /** 加载 chat pipeline；首次加载触发模型下载（通过 progress_callback 上报进度）；失败时清空 loading 以允许重试 */
  private async loadChatPipeline(onProgress?: (p: ChatModelProgress) => void): Promise<any> {
    if (!this.generator) {
      if (!this.chatLoading) {
        const id = this.chatModelId;
        this.chatLoading = (async () => {
          env.allowRemoteModels = true;
          env.allowLocalModels = false;
          await ensureRemoteHost(); // 官方源被墙时切换国内镜像
          return pipeline("text-generation", id, {
            dtype: "q8",
            progress_callback: onProgress
              ? (p: any) => {
                  const ev = toProgress(p);
                  if (ev) onProgress(ev);
                }
              : undefined,
          });
        })();
        this.chatLoading.catch(() => {
          this.chatLoading = null;
          this.chatState = "unavailable";
        });
      }
      this.generator = await this.chatLoading;
    }
    return this.generator;
  }

  async chat(messages: ChatMessage[]): Promise<string> {
    // 本地模型未缓存时抛出明确提示，避免静默下载数百 MB
    if (!this.generator && !this.chatLoading) {
      const cached = await isModelCached(this.chatModelId);
      if (!cached) {
        throw new Error("本地模型未下载，请在设置 → AI 能力中先下载本地模型");
      }
    }
    const generator = await this.loadChatPipeline();
    const out = await generator(messages, {
      max_new_tokens: 512,
      do_sample: false,
    });
    // v4 单输入时返回 { generated_text }，批量时返回数组；两种都兼容
    const text = Array.isArray(out) ? out[0]?.generated_text : out?.generated_text;
    if (typeof text !== "string" || !text.trim()) {
      throw new Error("本地模型未生成有效回复");
    }
    return text.trim();
  }
}

/** 余弦相似度（两个等长向量） */
export function cosine(a: number[], b: number[]): number {
  let dot = 0, na = 0, nb = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    na += a[i] * a[i];
    nb += b[i] * b[i];
  }
  if (na === 0 || nb === 0) return 0;
  return dot / (Math.sqrt(na) * Math.sqrt(nb));
}

/** 将长文本按行拆分为语义块（超长行按 maxLen 截断） */
export function chunkText(text: string, maxLen = 120): string[] {
  const re = new RegExp(`.{1,${maxLen}}`, "g");
  return text
    .split(/\n+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
    .flatMap((s) => (s.length > maxLen ? s.match(re) ?? [s] : [s]));
}

/**
 * 语义对比：A（原始）与 B（修改后）两组块向量，双向匹配：
 * - B 块在 A 中找到 > SIM_SAME 的匹配 → same；> SIM_REWRITE → rewritten；否则 → added
 * - A 块未被任何 B 块匹配 → removed
 */
export function semanticDiff(
  aVecs: number[][],
  aTexts: string[],
  bVecs: number[][],
  bTexts: string[]
) {
  const nA = aVecs.length;
  const nB = bVecs.length;
  /** 每个 B 块在 A 中的最佳匹配 [score, aIndex] */
  const bestB = new Array<[number, number]>(nB).fill([0, -1]);
  /** A 块是否已被 B 匹配 */
  const matchedA = new Array<boolean>(nA).fill(false);

  for (let j = 0; j < nB; j++) {
    let best = 0;
    let bestI = -1;
    for (let i = 0; i < nA; i++) {
      const s = cosine(bVecs[j], aVecs[i]);
      if (s > best) {
        best = s;
        bestI = i;
      }
    }
    bestB[j] = [best, bestI];
    if (best >= SIM_REWRITE && bestI >= 0) matchedA[bestI] = true;
  }

  const entries: { status: "same" | "rewritten" | "added" | "removed"; text: string; score: number }[] = [];
  for (let j = 0; j < nB; j++) {
    const [score, i] = bestB[j];
    if (i < 0 || score < SIM_REWRITE) {
      entries.push({ status: "added", text: bTexts[j], score });
    } else if (score >= SIM_SAME) {
      entries.push({ status: "same", text: bTexts[j], score });
    } else {
      entries.push({ status: "rewritten", text: bTexts[j], score });
    }
  }
  for (let i = 0; i < nA; i++) {
    if (!matchedA[i]) entries.push({ status: "removed", text: aTexts[i], score: 0 });
  }
  // 保持 B 侧顺序优先展示，removed 追加在末尾
  return entries;
}
