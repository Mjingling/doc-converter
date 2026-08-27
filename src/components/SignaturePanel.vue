<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("signature.title") }}</h2>
      <p>{{ t("signature.subtitle") }}</p>
    </div>

    <!-- PDF 上传区 -->
    <div class="upload-zone" @click="pickPdf" @dragover.prevent @drop.prevent="(e) => onDrop(e, handlePdf)">
      <div v-if="!file" class="zone-empty">
        <NIcon :component="CreateOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("signature.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#2080f0" />
        <span class="fname" :title="file">{{ fileName }}</span>
        <span v-if="pageCount > 0" class="size-tag">{{ t("pdfRender.pageCount", { n: pageCount }) }}</span>
        <button class="clear-btn" @click.stop="clearPdf">&times;</button>
      </div>
    </div>

    <!-- 签名图片上传区 -->
    <div class="upload-zone sig-zone" @click="pickSig" @dragover.prevent @drop.prevent="(e) => onDrop(e, handleSig)">
      <div v-if="!sig" class="zone-empty zone-compact">
        <NIcon :component="ImageOutline" :size="22" color="#f0a020" />
        <p class="zone-main">{{ t("signature.sigLabel") }}</p>
        <p class="zone-sub">{{ t("signature.sigDropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="ImageOutline" :size="20" color="#f0a020" />
        <span class="fname" :title="sig">{{ sigName }}</span>
        <span v-if="sigW > 0" class="size-tag">{{ sigW }} × {{ sigH }} px</span>
        <button class="clear-btn" @click.stop="clearSig">&times;</button>
      </div>
    </div>

    <div v-if="file && sig" class="config">
      <!-- 页码 -->
      <div class="config-row">
        <label class="config-label">{{ t("signature.pageLabel") }}</label>
        <NInputNumber v-model:value="page" :min="1" :max="pageCount > 0 ? pageCount : 9999" style="width: 140px" />
      </div>

      <!-- 位置预设 -->
      <div class="config-row">
        <label class="config-label">{{ t("signature.posLabel") }}</label>
        <NRadioGroup v-model:value="position">
          <div class="op-options">
            <NRadioButton value="tl" :label="t('signature.posTl')" />
            <NRadioButton value="tr" :label="t('signature.posTr')" />
            <NRadioButton value="bl" :label="t('signature.posBl')" />
            <NRadioButton value="br" :label="t('signature.posBr')" />
            <NRadioButton value="center" :label="t('signature.posCenter')" />
          </div>
        </NRadioGroup>
      </div>

      <!-- 宽度滑杆 -->
      <div class="config-row">
        <label class="config-label">{{ t("signature.widthLabel") }}：{{ width }}%</label>
        <NSlider v-model:value="width" :min="5" :max="50" :step="1" style="max-width: 360px" />
      </div>

      <!-- 输出路径预览 -->
      <div class="config-row">
        <label class="config-label">{{ t("signature.outLabel") }}</label>
        <span class="out-path" :title="outPath">{{ outPath }}</span>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("signature.hint") }}</span>
      <button class="cta" :disabled="!file || !sig || running" @click="run">
        <NIcon :component="CreateOutline" :size="17" />
        {{ running ? t("signature.running") : t("signature.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="running" :progress="running ? 0 : 100" :label="t('signature.running')" />

    <!-- 结果栏 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInputNumber, NRadioButton, NRadioGroup, NSlider, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { CreateOutline, DocumentTextOutline, ImageOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { getPdfPageCount, imageSize, pdfSign } from "../api";
import { extOf, defaultOutDir } from "../utils/file";
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
const sig = ref("");
const sigName = ref("");
const sigW = ref(0);
const sigH = ref(0);
const page = ref(1);
const position = ref<"tl" | "tr" | "bl" | "br" | "center">("br");
const width = ref(20);
const running = ref(false);
const resultOutputs = ref<string[]>([]);
const resultText = ref("");

/** 输出路径：源目录（或默认输出目录）下 原名_signed.pdf */
const outPath = computed(() => {
  if (!file.value) return "";
  const base = file.value.split(/[\\/]/).pop()?.replace(/\.pdf$/i, "") ?? "output";
  return `${defaultOutDir(file.value, settings.defaultOutDir)}/${base}_signed.pdf`;
});

async function handlePdf(path: string) {
  if (extOf(path).toLowerCase() !== "pdf") {
    message.warning(t("signature.warnOnlyPdf"));
    return;
  }
  file.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
  pageCount.value = 0;
  page.value = 1;
  resultText.value = "";
  resultOutputs.value = [];
  try {
    pageCount.value = await getPdfPageCount(path);
  } catch {
    // 页数获取失败不阻断，后端会校验页码
  }
}

async function handleSig(path: string) {
  const ext = extOf(path).toLowerCase();
  if (!["png", "jpg", "jpeg"].includes(ext)) {
    message.warning(t("signature.warnOnlyImage"));
    return;
  }
  sig.value = path;
  sigName.value = path.split(/[/\\]/).pop() || path;
  sigW.value = 0;
  sigH.value = 0;
  try {
    const [w, h] = await imageSize(path);
    sigW.value = w;
    sigH.value = h;
  } catch {
    // 尺寸获取失败时按正方形近似换算
  }
}

async function pickPdf() {
  const sel = await open({ filters: [{ name: "PDF", extensions: ["pdf"] }], multiple: false });
  if (sel) await handlePdf(String(sel));
}

async function pickSig() {
  const sel = await open({
    filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg"] }],
    multiple: false,
  });
  if (sel) await handleSig(String(sel));
}

function onDrop(e: DragEvent, handler: (path: string) => Promise<void>) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) void handler((f as any).path);
}

function clearPdf() {
  file.value = "";
  fileName.value = "";
  pageCount.value = 0;
}

function clearSig() {
  sig.value = "";
  sigName.value = "";
  sigW.value = 0;
  sigH.value = 0;
}

/**
 * 位置预设 → 页面百分比坐标（PDF 原点左下，y 为图片底边距底部的百分比）
 * 高度百分比按图片宽高比近似换算（页面按 A4 竖版 595×842 估算）
 */
function calcPos(): { x: number; y: number } {
  const w = width.value;
  const ratio = sigW.value > 0 && sigH.value > 0 ? sigH.value / sigW.value : 0.35;
  const h = Math.min(w * ratio * (595 / 842), 100);
  const m = 5; // 边距百分比
  switch (position.value) {
    case "tl": return { x: m, y: Math.max(0, 100 - h - m) };
    case "tr": return { x: Math.max(0, 100 - w - m), y: Math.max(0, 100 - h - m) };
    case "bl": return { x: m, y: m };
    case "br": return { x: Math.max(0, 100 - w - m), y: m };
    case "center": return { x: Math.max(0, (100 - w) / 2), y: Math.max(0, (100 - h) / 2) };
  }
}

async function run() {
  if (!file.value || !sig.value || running.value) return;
  if (pageCount.value > 0 && (page.value < 1 || page.value > pageCount.value)) {
    message.warning(t("signature.pageInvalid", { n: pageCount.value }));
    return;
  }
  running.value = true;
  try {
    const { x, y } = calcPos();
    const out = await pdfSign(file.value, outPath.value, sig.value, page.value, x, y, width.value);
    await history.add({
      kind: "signature",
      name: fileName.value,
      inputs: [file.value, sig.value],
      outputs: [out],
      ok: true,
    });
    resultText.value = t("signature.success");
    resultOutputs.value = [out];
    void notifyDone(t("common.taskDone"), t("signature.success"));
  } catch (e: any) {
    message.error(t("signature.fail", { err: String(e) }));
  } finally {
    running.value = false;
  }
}

defineExpose({ handleDrop: (paths: string[]) => paths[0] && handlePdf(paths[0]) });
</script>

<style scoped>
.panel { background: var(--bg-panel); border-radius: 14px; padding: 30px; box-shadow: 0 1px 3px var(--shadow); }
.panel-head h2 { margin: 0; font-size: 22px; color: var(--text-main); }
.panel-head p { margin: 6px 0 0; font-size: 13px; color: var(--text-muted); }
.upload-zone { margin-top: 18px; border: 2px dashed var(--border-dash); border-radius: 14px; padding: 28px; text-align: center; cursor: pointer; transition: all 0.2s; }
.upload-zone:hover { border-color: var(--accent); background: var(--accent-soft); }
.sig-zone { padding: 18px; }
.zone-empty { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.zone-compact { flex-direction: row; gap: 12px; }
.zone-compact p { margin: 0; }
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
.op-options { display: flex; gap: 10px; flex-wrap: wrap; }
.out-path { font-size: 12px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>
