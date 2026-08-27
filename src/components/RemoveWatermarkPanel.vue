<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("removeWatermark.title") }}</h2>
      <p>{{ t("removeWatermark.subtitle") }}</p>
    </div>

    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="!filePath" class="zone-empty">
        <NIcon :component="WaterOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("removeWatermark.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="filePath">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <div class="action-row">
      <span class="hint">{{ t("removeWatermark.hint") }}</span>
      <button class="cta" :disabled="!filePath || running" @click="run">
        <NIcon :component="WaterOutline" :size="17" />
        {{ running ? t("removeWatermark.running") : t("removeWatermark.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="running" indeterminate :label="t('removeWatermark.running')" />

    <div v-if="resultPath" class="results">
      <p class="result-title">{{ t("removeWatermark.success", { name: resultName }) }}</p>
      <div class="result-item">
        <span>{{ resultPath }}</span>

        <span class="result-actions">
          <NButton size="tiny" secondary type="primary" @click="openPath(resultPath)">{{ t("common.open") }}</NButton>
          <NButton size="tiny" @click="openPath(dirOf(resultPath))">{{ t("common.openDir") }}</NButton>
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon, NButton } from "naive-ui";
import { useI18n } from "vue-i18n";
import { WaterOutline, DocumentTextOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { pdfRemoveWatermark, openPath } from "../api";
import { dirOf, defaultOutputPath } from "../utils/file";
import TaskProgress from "./TaskProgress.vue";
import { usePanelTask } from "../composables/usePanelTask";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();
const settings = useSettingsStore();

const filePath = ref("");
const fileName = ref("");
const resultPath = ref("");
const resultName = ref("");

/** 执行状态：running + 进度条（handler 名 run，解构重命名避免冲突） */
const { running, run: runTask } = usePanelTask();

function handleFile(path: string) {
  if (!path.toLowerCase().endsWith(".pdf")) {
    message.warning(t("removeWatermark.warnOnlyPdf"));
    return;
  }
  filePath.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
  resultPath.value = "";
}

function clearFile() {
  filePath.value = "";
  fileName.value = "";
  resultPath.value = "";
}

async function pickFile() {
  const sel = await open({ filters: [{ name: "PDF", extensions: ["pdf"] }], multiple: false });
  if (sel) handleFile(sel);
}

function onDrop(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile((f as any).path);
}

async function run() {
  if (!filePath.value) return;
  const outPath = defaultOutputPath(filePath.value, "_no_watermark", settings.defaultOutDir);
  await runTask(async () => {
    try {
      const out = await pdfRemoveWatermark(filePath.value, outPath);
      resultPath.value = out;
      resultName.value = out.split(/[/\\]/).pop() || out;
      history.add({ kind: "removeWatermark", name: fileName.value, inputs: [filePath.value], outputs: [out], ok: true });
      message.success(t("removeWatermark.success", { name: resultName.value }));
    } catch (e: any) {
      history.add({ kind: "removeWatermark", name: fileName.value, inputs: [filePath.value], outputs: [], ok: false });
      message.error(t("removeWatermark.fail", { err: e }));
    }
  });
}

defineExpose({ handleDrop: handleFile });
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
.fname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: var(--text-body); }
.size-tag { font-size: 11px; padding: 2px 8px; border-radius: 8px; color: var(--text-muted); background: var(--bg-tag); }
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
.results { margin-top: 16px; }
.result-title { font-size: 14px; font-weight: 600; color: var(--green); margin-bottom: 8px; }
.result-item { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 6px 0; border-bottom: 1px solid var(--border-soft); font-size: 12px; color: var(--text-sub); }
.result-item > span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.result-actions { display: flex; gap: 6px; flex-shrink: 0; }
</style>
