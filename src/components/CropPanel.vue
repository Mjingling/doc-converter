<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("crop.title") }}</h2>
      <p>{{ t("crop.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <input ref="fileInput" type="file" accept=".pdf" style="display:none" @change="onFileChange" />
      <div v-if="!filePath" class="zone-empty">
        <NIcon :component="ResizeOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("crop.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="filePath">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <div v-if="filePath" class="form">
      <div class="field-row">
        <div class="field">
          <label>{{ t("crop.topLabel") }}</label>
          <NInputNumber v-model:value="topVal" :min="-500" :max="500" placeholder="0" />
        </div>
        <div class="field">
          <label>{{ t("crop.bottomLabel") }}</label>
          <NInputNumber v-model:value="bottomVal" :min="-500" :max="500" placeholder="0" />
        </div>
      </div>
      <div class="field-row">
        <div class="field">
          <label>{{ t("crop.leftLabel") }}</label>
          <NInputNumber v-model:value="leftVal" :min="-500" :max="500" placeholder="0" />
        </div>
        <div class="field">
          <label>{{ t("crop.rightLabel") }}</label>
          <NInputNumber v-model:value="rightVal" :min="-500" :max="500" placeholder="0" />
        </div>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("crop.hint") }}</span>
      <button class="cta" :disabled="!filePath" @click="run">
        <NIcon :component="ResizeOutline" :size="17" />
        {{ t("crop.cta") }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon, NInputNumber } from "naive-ui";
import { useI18n } from "vue-i18n";
import { DocumentTextOutline, ResizeOutline } from "@vicons/ionicons5";
import { save } from "@tauri-apps/plugin-dialog";
import { pdfCrop } from "../api";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();

const filePath = ref("");
const fileName = ref("");
const leftVal = ref(0);
const rightVal = ref(0);
const topVal = ref(0);
const bottomVal = ref(0);

function handleFile(path: string) {
  if (!path.toLowerCase().endsWith(".pdf")) {
    message.warning(t("crop.warnOnlyPdf"));
    return;
  }
  filePath.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
}

function clearFile() {
  filePath.value = "";
  fileName.value = "";
  leftVal.value = 0; rightVal.value = 0; topVal.value = 0; bottomVal.value = 0;
}

async function pickFile() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const sel = await open({ filters: [{ name: "PDF", extensions: ["pdf"] }], multiple: false });
  if (sel) handleFile(sel);
}

function onDrop(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile((f as any).path);
}

function onFileChange(e: Event) {
  const input = e.target as HTMLInputElement;
  const f = input.files?.[0];
  if (f) handleFile((f as any).path);
}

async function run() {
  if (!filePath.value) return;
  const out = await save({
    filters: [{ name: "PDF", extensions: ["pdf"] }],
    defaultPath: fileName.value.replace(".pdf", "_cropped.pdf"),
  });
  if (!out) return;
  try {
    await pdfCrop(filePath.value, out, leftVal.value, bottomVal.value, rightVal.value, topVal.value);
    history.add({ kind: "crop", name: fileName.value, inputs: [filePath.value], outputs: [out], ok: true });
    message.success(t("crop.success", { name: fileName.value }));
  } catch (e: any) {
    history.add({ kind: "crop", name: fileName.value, inputs: [filePath.value], outputs: [], ok: false });
    message.error(t("crop.fail", { err: e }));
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
.field-row { display: flex; gap: 14px; }
.field { flex: 1; display: flex; flex-direction: column; gap: 8px; }
.field label { font-size: 13px; color: var(--text-sub); }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>