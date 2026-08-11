<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("docxExtract.title") }}</h2>
      <p>{{ t("docxExtract.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="!filePath" class="zone-empty">
        <NIcon :component="DocumentAttachOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("docxExtract.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="filePath">{{ fileName }}</span>
        <span class="size-tag">DOCX</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <div v-if="filePath" class="form">
      <div class="field">
        <label>{{ t("split.outDirLabel") }}</label>
        <div class="dir-row">
          <NInput :value="outDir || t('split.outDirPlaceholder')" readonly :placeholder="t('split.outDirPlaceholder')" />
          <NButton size="small" @click="pickDir">{{ t("common.open") }}</NButton>
        </div>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("docxExtract.hint") }}</span>
      <button class="cta" :disabled="!outDir" @click="run">
        <NIcon :component="DocumentAttachOutline" :size="17" />
        {{ t("docxExtract.cta") }}
      </button>
    </div>

    <div v-if="results.length" class="results">
      <p class="result-title">{{ t("docxExtract.success", { n: results.length, dir: outDir }) }}</p>
      <div v-for="(r, i) in results" :key="i" class="result-item">
        <span>{{ r }}</span>
        <NButton size="tiny" @click="openPath(dirOf(r))">{{ t("common.openDir") }}</NButton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon, NInput, NButton } from "naive-ui";
import { useI18n } from "vue-i18n";
import { DocumentAttachOutline, DocumentTextOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { docxExtractImages, openPath } from "../api";
import { dirOf } from "../utils/file";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();

const filePath = ref("");
const fileName = ref("");
const outDir = ref("");
const results = ref<string[]>([]);

function handleFile(path: string) {
  if (!path.toLowerCase().endsWith(".docx")) {
    message.warning(t("docxExtract.warnOnly"));
    return;
  }
  filePath.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
  results.value = [];
}

function clearFile() {
  filePath.value = "";
  fileName.value = "";
  outDir.value = "";
  results.value = [];
}

async function pickFile() {
  const sel = await open({ filters: [{ name: "Word Document", extensions: ["docx"] }], multiple: false });
  if (sel) handleFile(sel);
}

function onDrop(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile((f as any).path);
}

async function pickDir() {
  const sel = await open({ directory: true, title: t("docxExtract.pickDirTitle") });
  if (sel) outDir.value = sel;
}

async function run() {
  if (!filePath.value || !outDir.value) return;
  try {
    const imgs = await docxExtractImages(filePath.value, outDir.value);
    results.value = imgs;
    history.add({ kind: "docxExtract", name: fileName.value, inputs: [filePath.value], outputs: [outDir.value], ok: true });
    message.success(t("docxExtract.success", { n: imgs.length, dir: outDir.value }));
  } catch (e: any) {
    history.add({ kind: "docxExtract", name: fileName.value, inputs: [filePath.value], outputs: [], ok: false });
    message.error(t("docxExtract.fail", { err: e }));
  }
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
.form { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.field { display: flex; flex-direction: column; gap: 8px; }
.field label { font-size: 13px; color: var(--text-sub); }
.dir-row { display: flex; gap: 8px; }
.dir-row .n-input { flex: 1; }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
.results { margin-top: 16px; }
.result-title { font-size: 14px; font-weight: 600; color: var(--green); margin-bottom: 8px; }
.result-item { display: flex; align-items: center; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--border-soft); font-size: 12px; color: var(--text-sub); }
</style>