<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("comparePdf.title") }}</h2>
      <p>{{ t("comparePdf.subtitle") }}</p>
    </div>

    <!-- 模式切换：精确（文本 diff）/ 语义（AI embedding） -->
    <div class="mode-switch">
      <button
        class="mode-btn"
        :class="{ active: mode === 'exact' }"
        @click="mode = 'exact'; semDiffs = []"
      >{{ t("comparePdf.modeExact") }}</button>
      <button
        class="mode-btn"
        :class="{ active: mode === 'semantic' }"
        @click="mode = 'semantic'; semDiffs = []"
      >{{ t("comparePdf.modeSemantic") }}</button>
      <span v-if="mode === 'semantic'" class="model-status" :class="modelState">
        {{ modelState === 'loading' ? t("comparePdf.semLoading") : modelState === 'ready' ? t("comparePdf.semReady") : '' }}
      </span>
    </div>

    <!-- 语义模型预热失败：提供重试入口 -->
    <div v-if="mode === 'semantic' && modelState === 'unavailable'" class="warm-retry">
      <button class="retry-btn" @click="warmModel">{{ t("comparePdf.semRetry") }}</button>
    </div>

    <div class="two-col">
      <div class="upload-zone" @click="pickFile('a')" @dragover.prevent @drop.prevent="onDropA">
        <div v-if="!fileA" class="zone-empty">
          <NIcon :component="DocumentTextOutline" :size="28" color="#e6494c" />
          <p class="zone-main">{{ t("comparePdf.fileALabel") }}</p>
          <p class="zone-sub">{{ t("comparePdf.dropSub") }}</p>
        </div>
        <div v-else class="zone-filled">
          <NIcon :component="DocumentTextOutline" :size="18" color="#e6494c" />
          <span class="fname" :title="fileA">{{ fileNameA }}</span>
          <button class="clear-btn" @click.stop="fileA = ''; fileNameA = ''; diffs = []; semDiffs = []">&times;</button>
        </div>
      </div>

      <div class="upload-zone" @click="pickFile('b')" @dragover.prevent @drop.prevent="onDropB">
        <div v-if="!fileB" class="zone-empty">
          <NIcon :component="DocumentTextOutline" :size="28" color="#2080f0" />
          <p class="zone-main">{{ t("comparePdf.fileBLabel") }}</p>
          <p class="zone-sub">{{ t("comparePdf.dropSub") }}</p>
        </div>
        <div v-else class="zone-filled">
          <NIcon :component="DocumentTextOutline" :size="18" color="#2080f0" />
          <span class="fname" :title="fileB">{{ fileNameB }}</span>
          <button class="clear-btn" @click.stop="fileB = ''; fileNameB = ''; diffs = []; semDiffs = []">&times;</button>
        </div>
      </div>
    </div>

    <div class="action-row">
      <span class="hint">{{ t("comparePdf.hint") }}</span>
      <button class="cta" :disabled="!fileA || !fileB || loading" @click="run">
        <NIcon :component="DocumentTextOutline" :size="17" />
        {{
          loading
            ? (mode === 'semantic' ? t("comparePdf.semAnalyzing") : t("common.converting"))
            : (mode === 'semantic' ? t("comparePdf.semAnalyze") : t("comparePdf.cta"))
        }}
      </button>
    </div>

    <!-- 精确模式：行级 diff -->
    <div v-if="mode === 'exact' && diffs.length > 0" class="diff-results">
      <p class="result-title">{{ t("comparePdf.resultTitle", { n: diffs.length }) }}</p>
      <div class="diff-table">
        <div v-for="(d, i) in diffs" :key="i" class="diff-row" :class="d.status">
          <span class="diff-status">{{ d.status === 'added' ? '+' : d.status === 'removed' ? '-' : ' ' }}</span>
          <span class="diff-line">{{ d.line }}</span>
        </div>
      </div>
    </div>

    <!-- 语义模式：AI 分块对比结果 -->
    <div v-if="mode === 'semantic' && semDiffs.length > 0" class="diff-results">
      <p class="result-title">
        {{ t("comparePdf.semSummary", { n: semDiffs.length, same: semCounts.same, rewritten: semCounts.rewritten, added: semCounts.added, removed: semCounts.removed }) }}
      </p>
      <div class="diff-table">
        <div
          v-for="(d, i) in semDiffs"
          :key="i"
          class="diff-row"
          :class="'sem-' + d.status"
        >
          <span class="sem-badge" :class="d.status">{{ semStatusLabel(d.status) }}</span>
          <span class="diff-line">{{ d.text }}</span>
          <span v-if="d.score > 0" class="sem-score">{{ t("comparePdf.semScore", { s: Math.round(d.score * 100) }) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { useMessage, NIcon } from "naive-ui";
import { useI18n } from "vue-i18n";
import { DocumentTextOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { pdfCompare, pdfExtractText, type DiffEntry } from "../api";
import { useHistoryStore } from "../stores/history";
import { embed, chunkText, semanticDiff } from "../ai";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();

type Mode = "exact" | "semantic";
const mode = ref<Mode>("exact");

const fileA = ref("");
const fileNameA = ref("");
const fileB = ref("");
const fileNameB = ref("");
const diffs = ref<DiffEntry[]>([]);
const semDiffs = ref<{ status: "same" | "rewritten" | "added" | "removed"; text: string; score: number }[]>([]);
const loading = ref(false);

/** 本地 AI 模型状态：unavailable（未初始化）/ loading / ready */
const modelState = ref<"unavailable" | "loading" | "ready">("unavailable");

const semCounts = computed(() => {
  const c = { same: 0, rewritten: 0, added: 0, removed: 0 };
  for (const d of semDiffs.value) c[d.status]++;
  return c;
});

function semStatusLabel(status: string): string {
  switch (status) {
    case "same": return t("comparePdf.semStatusSame");
    case "rewritten": return t("comparePdf.semStatusRewritten");
    case "added": return t("comparePdf.semStatusAdded");
    case "removed": return t("comparePdf.semStatusRemoved");
    default: return "";
  }
}

function handleFile(side: "a" | "b", path: string) {
  if (!path.toLowerCase().endsWith(".pdf")) {
    message.warning(t("comparePdf.warnOnlyPdf"));
    return;
  }
  const name = path.split(/[/\\]/).pop() || path;
  if (side === "a") {
    fileA.value = path;
    fileNameA.value = name;
  } else {
    fileB.value = path;
    fileNameB.value = name;
  }
  diffs.value = [];
  semDiffs.value = [];
}

async function pickFile(side: "a" | "b") {
  const sel = await open({ filters: [{ name: "PDF", extensions: ["pdf"] }], multiple: false });
  if (sel) handleFile(side, sel);
}

function onDropA(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile("a", (f as any).path);
}

function onDropB(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile("b", (f as any).path);
}

async function run() {
  if (!fileA.value || !fileB.value) return;
  if (mode.value === "exact") return runExact();
  await runSemantic();
}

/** 精确模式：后端行级文本 diff */
async function runExact() {
  loading.value = true;
  try {
    const result = await pdfCompare(fileA.value, fileB.value);
    diffs.value = result;
    history.add({
      kind: "comparePdf",
      name: `${fileNameA.value} vs ${fileNameB.value}`,
      inputs: [fileA.value, fileB.value],
      outputs: [],
      ok: true,
    });
    message.success(t("comparePdf.success", { n: result.length }));
  } catch (e: any) {
    history.add({
      kind: "comparePdf",
      name: `${fileNameA.value} vs ${fileNameB.value}`,
      inputs: [fileA.value, fileB.value],
      outputs: [],
      ok: false,
    });
    message.error(t("comparePdf.fail", { err: e }));
  } finally {
    loading.value = false;
  }
}

/** 预热本地 AI 模型；失败时返回 false（面板显示重试入口） */
async function warmModel(): Promise<boolean> {
  modelState.value = "loading";
  try {
    await embed(["预热"]);
    modelState.value = "ready";
    return true;
  } catch (e: any) {
    modelState.value = "unavailable";
    message.error(t("comparePdf.semFail", { err: e }));
    return false;
  }
}

/** 语义模式：本地 AI embedding + 相似度匹配 */
async function runSemantic() {
  loading.value = true;
  try {
    // 1. 提取两个 PDF 全文
    const [textA, textB] = await Promise.all([
      pdfExtractText(fileA.value),
      pdfExtractText(fileB.value),
    ]);
    if (!textA.trim() || !textB.trim()) {
      message.warning(t("comparePdf.semNoText"));
      return;
    }

    // 2. 预热本地模型（首次触发下载 + 初始化）；auto 模式下自动回退云端
    if (modelState.value !== "ready") {
      const warmed = await warmModel();
      if (!warmed) return;
    }

    // 3. 分块 + embedding + 相似度匹配（限制块数，避免长文档 embed 数千块导致性能崩溃或 API 超限）
    const MAX_CHUNKS = 500;
    let aChunks = chunkText(textA);
    let bChunks = chunkText(textB);
    if (aChunks.length > MAX_CHUNKS) {
      aChunks = aChunks.slice(0, MAX_CHUNKS);
    }
    if (bChunks.length > MAX_CHUNKS) {
      bChunks = bChunks.slice(0, MAX_CHUNKS);
    }
    const [aVecs, bVecs] = await Promise.all([
      embed(aChunks),
      embed(bChunks),
    ]);
    semDiffs.value = semanticDiff(aVecs, aChunks, bVecs, bChunks);

    history.add({
      kind: "comparePdf",
      name: `${fileNameA.value} vs ${fileNameB.value}（语义）`,
      inputs: [fileA.value, fileB.value],
      outputs: [],
      ok: true,
    });
  } catch (e: any) {
    history.add({
      kind: "comparePdf",
      name: `${fileNameA.value} vs ${fileNameB.value}（语义）`,
      inputs: [fileA.value, fileB.value],
      outputs: [],
      ok: false,
    });
    message.error(t("comparePdf.semFail", { err: e }));
  } finally {
    loading.value = false;
  }
}

defineExpose({ handleDrop: (paths: string[]) => {
  if (paths.length >= 1) handleFile("a", paths[0]);
  if (paths.length >= 2) handleFile("b", paths[1]);
}});
</script>

<style scoped>
.panel { background: var(--bg-panel); border-radius: 14px; padding: 30px; box-shadow: 0 1px 3px var(--shadow); }
.panel-head h2 { margin: 0; font-size: 22px; color: var(--text-main); }
.panel-head p { margin: 6px 0 0; font-size: 13px; color: var(--text-muted); }
.mode-switch { display: flex; align-items: center; gap: 8px; margin-top: 16px; }
.warm-retry { margin-top: 10px; }
.retry-btn { border: 1px solid var(--border-strong); background: var(--bg-panel); color: var(--accent); font-size: 12px; padding: 5px 14px; border-radius: 6px; cursor: pointer; transition: all 0.15s; }
.retry-btn:hover { border-color: var(--accent); }
.mode-btn { border: 1px solid var(--border-soft); background: var(--bg-page); color: var(--text-sub); font-size: 13px; padding: 6px 16px; border-radius: 20px; cursor: pointer; transition: all 0.15s; }
.mode-btn:hover { border-color: var(--accent); color: var(--accent); }
.mode-btn.active { background: var(--accent); border-color: var(--accent); color: var(--cta-text); font-weight: 600; }
.model-status { font-size: 12px; color: var(--text-muted); }
.model-status.loading { color: var(--accent); }
.model-status.ready { color: var(--green); }
.two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 18px; }
.upload-zone { border: 2px dashed var(--border-dash); border-radius: 14px; padding: 24px; text-align: center; cursor: pointer; transition: all 0.2s; }
.upload-zone:hover { border-color: var(--accent); background: var(--accent-soft); }
.zone-empty { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.zone-main { margin: 0; font-size: 14px; color: var(--text-sub); }
.zone-sub { margin: 0; font-size: 12px; color: var(--text-faint); }
.zone-filled { display: flex; align-items: center; justify-content: center; gap: 8px; font-size: 14px; color: var(--text-sub); }
.fname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: var(--text-body); }
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
.diff-results { margin-top: 16px; }
.result-title { font-size: 14px; font-weight: 600; color: var(--green); margin-bottom: 8px; }
.diff-table { max-height: 400px; overflow-y: auto; border: 1px solid var(--border-soft); border-radius: 8px; }
.diff-row { display: flex; align-items: flex-start; gap: 8px; padding: 6px 12px; font-family: monospace; font-size: 12px; border-bottom: 1px solid var(--border-soft); }
.diff-row:last-child { border-bottom: none; }
.diff-row.added { background: #e6ffed; color: #1a7f37; }
.diff-row.removed { background: #ffeef0; color: #cf222e; }
.diff-row.unchanged { color: var(--text-muted); }
.diff-status { font-weight: 700; width: 14px; flex-shrink: 0; }
.diff-line { white-space: pre-wrap; word-break: break-all; flex: 1; }
.sem-badge { flex-shrink: 0; font-size: 11px; font-weight: 600; padding: 1px 8px; border-radius: 10px; margin-top: 1px; }
.sem-badge.same { background: #e6ffed; color: #1a7f37; }
.sem-badge.rewritten { background: #fff8c5; color: #9a6700; }
.sem-badge.added { background: #ddf4ff; color: #0969da; }
.sem-badge.removed { background: #ffeef0; color: #cf222e; }
.diff-row.sem-rewritten { background: #fffdf0; }
.diff-row.sem-added { background: #f6fbff; }
.sem-score { flex-shrink: 0; font-size: 11px; color: var(--text-muted); }
</style>
