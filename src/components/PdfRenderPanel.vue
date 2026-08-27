<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("pdfRender.title") }}</h2>
      <p>{{ t("pdfRender.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="!file" class="zone-empty">
        <NIcon :component="ImageOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("pdfRender.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#2080f0" />
        <span class="fname" :title="file">{{ fileName }}</span>
        <span v-if="pageCount > 0" class="size-tag">{{ t("pdfRender.pageCount", { n: pageCount }) }}</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <div v-if="file" class="config">
      <!-- 页码选择 -->
      <div class="config-row">
        <label class="config-label">{{ t("pdfRender.pagesLabel") }}</label>
        <NRadioGroup v-model:value="pageMode">
          <div class="op-options">
            <NRadioButton value="all" :label="t('pdfRender.pagesAll')" />
            <NRadioButton value="range" :label="t('pdfRender.pagesRange')" />
          </div>
        </NRadioGroup>
        <NInput
          v-if="pageMode === 'range'"
          v-model:value="rangeText"
          :placeholder="t('pdfRender.rangePlaceholder')"
          style="margin-top: 8px"
        />
      </div>

      <!-- 输出格式 -->
      <div class="config-row">
        <label class="config-label">{{ t("pdfRender.formatLabel") }}</label>
        <NRadioGroup v-model:value="format">
          <div class="op-options">
            <NRadioButton value="png" label="PNG" />
            <NRadioButton value="jpg" label="JPG" />
          </div>
        </NRadioGroup>
      </div>

      <!-- DPI -->
      <div class="config-row">
        <label class="config-label">{{ t("pdfRender.dpiLabel") }}</label>
        <NRadioGroup v-model:value="dpi">
          <div class="op-options">
            <NRadioButton v-for="d in [72, 150, 300]" :key="d" :value="d" :label="`${d} DPI`" />
          </div>
        </NRadioGroup>
        <span class="config-hint">{{ t("pdfRender.dpiHint") }}</span>
      </div>

      <!-- 输出目录预览 -->
      <div class="config-row">
        <label class="config-label">{{ t("pdfRender.outLabel") }}</label>
        <span class="out-path" :title="outDir">{{ outDir }}</span>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("pdfRender.hint") }}</span>
      <button class="cta" :disabled="!file || running" @click="run">
        <NIcon :component="ImageOutline" :size="17" />
        {{ running ? t("pdfRender.running") : t("pdfRender.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="running" :progress="running ? 0 : 100" :label="t('pdfRender.running')" />

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInput, NRadioButton, NRadioGroup, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { DocumentTextOutline, ImageOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { getPdfPageCount, pdfRender } from "../api";
import { extOf, defaultOutDir } from "../utils/file";
import { parsePageRanges } from "../utils/pageRanges";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";
import { notifyDone } from "../utils/notify";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const history = useHistoryStore();

const file = ref("");
const fileName = ref("");
const pageCount = ref(0);
const pageMode = ref<"all" | "range">("all");
const rangeText = ref("");
const format = ref("png");
const dpi = ref(150);
const running = ref(false);
const resultOutputs = ref<string[]>([]);
const resultText = ref("");

/** 输出目录：源目录（或默认输出目录）下同名 _rendered 子目录 */
const outDir = computed(() => {
  if (!file.value) return "";
  const base = file.value.split(/[\\/]/).pop()?.replace(/\.pdf$/i, "") ?? "output";
  return `${defaultOutDir(file.value, settings.defaultOutDir)}/${base}_rendered`;
});

async function handleFile(path: string) {
  if (extOf(path).toLowerCase() !== "pdf") {
    message.warning(t("pdfRender.warnOnlyPdf"));
    return;
  }
  file.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
  pageCount.value = 0;
  resultText.value = "";
  resultOutputs.value = [];
  try {
    pageCount.value = await getPdfPageCount(path);
  } catch {
    // 页数获取失败不阻断，渲染时由后端校验
  }
}

async function pickFile() {
  const sel = await open({ filters: [{ name: "PDF", extensions: ["pdf"] }], multiple: false });
  if (sel) await handleFile(String(sel));
}

function onDrop(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) void handleFile((f as any).path);
}

function clearFile() {
  file.value = "";
  fileName.value = "";
  pageCount.value = 0;
}

async function run() {
  if (!file.value || running.value) return;
  // 指定页码模式：前端先按已知总页数校验（未知总页数时交给后端校验）
  let pages: number[] | null = null;
  if (pageMode.value === "range") {
    try {
      pages = parsePageRanges(rangeText.value, pageCount.value || Number.MAX_SAFE_INTEGER);
    } catch {
      message.warning(t("pdfRender.rangeInvalid", { n: pageCount.value }));
      return;
    }
  }
  running.value = true;
  try {
    const outs = await pdfRender(file.value, outDir.value, pages, format.value, dpi.value);
    await history.add({
      kind: "pdfRender",
      name: fileName.value,
      inputs: [file.value],
      outputs: outs,
      ok: outs.length > 0,
    });
    resultText.value = t("pdfRender.success", { n: outs.length });
    resultOutputs.value = outs;
    void notifyDone(t("common.taskDone"), t("pdfRender.success", { n: outs.length }));
  } catch (e: any) {
    message.error(t("pdfRender.fail", { err: String(e) }));
  } finally {
    running.value = false;
  }
}

defineExpose({ handleDrop: (paths: string[]) => paths[0] && handleFile(paths[0]) });
</script>

<style scoped>
.panel { background: var(--bg-panel); border-radius: 14px; padding: 30px; box-shadow: 0 1px 3px var(--shadow); }
.panel-head h2 { margin: 0; font-size: 22px; color: var(--text-main); }
.panel-head p { margin: 6px 0 0; font-size: 13px; color: var(--text-muted); }
.upload-zone { margin-top: 18px; border: 2px dashed var(--border-dash); border-radius: 14px; padding: 28px; text-align: center; cursor: pointer; transition: all 0.2s; }
.upload-zone:hover { border-color: var(--accent); background: var(--accent-soft); }
.zone-empty { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.zone-main { margin: 0; font-size: 14px; color: var(--text-sub); }
.zone-main .link { color: var(--accent); cursor: pointer; }
.zone-sub { margin: 0; font-size: 12px; color: var(--text-faint); }
.zone-filled { display: flex; align-items: center; justify-content: center; gap: 10px; font-size: 14px; color: var(--text-sub); }
.fname { max-width: 380px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.size-tag { font-size: 12px; color: var(--text-muted); background: var(--bg-tag); border-radius: 6px; padding: 2px 8px; }
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.config { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.config-row { display: flex; flex-direction: column; gap: 8px; }
.config-label { font-size: 13px; color: var(--text-sub); }
.config-hint { font-size: 12px; color: var(--text-muted); }
.op-options { display: flex; gap: 10px; flex-wrap: wrap; }
.out-path { font-size: 12px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>
