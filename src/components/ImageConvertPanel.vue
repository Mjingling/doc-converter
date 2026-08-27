<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("imageConvert.title") }}</h2>
      <p>{{ t("imageConvert.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFiles" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="files.length === 0" class="zone-empty">
        <NIcon :component="ImagesOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("imageConvert.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="CloudUploadOutline" :size="20" color="#2080f0" />
        <span>{{ t("imageConvert.added", { n: files.length }) }}</span>
        <button class="clear-btn" @click.stop="clearFiles">&times;</button>
      </div>
    </div>

    <div v-if="files.length > 0" class="form">
      <!-- 目标格式 -->
      <div class="field">
        <label>{{ t("imageConvert.formatLabel") }}</label>
        <NRadioGroup v-model:value="format">
          <div class="op-options">
            <NRadioButton v-for="f in formats" :key="f" :value="f" :label="f.toUpperCase()" />
          </div>
        </NRadioGroup>
      </div>

      <!-- 可选缩放：宽或高留 0 表示按比例 -->
      <div class="field">
        <label>{{ t("imageConvert.resizeLabel") }}</label>
        <div class="size-row">
          <NInputNumber v-model:value="width" :min="0" :max="10000" :placeholder="t('imageConvert.widthPh')" />
          <span class="size-x">&times;</span>
          <NInputNumber v-model:value="height" :min="0" :max="10000" :placeholder="t('imageConvert.heightPh')" />
        </div>
        <span class="hint">{{ t("imageConvert.resizeHint") }}</span>
      </div>

      <!-- JPEG 质量（仅 jpg 目标显示） -->
      <div v-if="format === 'jpg'" class="field">
        <label>{{ t("imageConvert.qualityLabel") }}: {{ quality }}</label>
        <NSlider v-model:value="quality" :min="10" :max="100" :step="5" />
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("imageConvert.outHint") }}</span>
      <button class="cta" :disabled="files.length === 0 || running" @click="run">
        <NIcon :component="ImagesOutline" :size="17" />
        {{ running ? t("imageConvert.running", { done, total: files.length }) : t("imageConvert.cta") }}
      </button>
    </div>

    <!-- 执行进度：多文件真实百分比 -->
    <TaskProgress
      :running="running"
      :progress="Math.round((done / files.length) * 100)"
      :label="t('imageConvert.running', { done, total: files.length })"
    />

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NIcon, NInputNumber, NRadioButton, NRadioGroup, NSlider, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { CloudUploadOutline, ImagesOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { extOf, defaultOutDir } from "../utils/file";
import { imageConvert, imageResize } from "../api";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";
import { notifyDone } from "../utils/notify";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();
const settings = useSettingsStore();
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");

const IMAGE_EXTS = ["jpg", "jpeg", "png", "webp", "bmp", "gif"];
const formats = ["png", "jpg", "webp", "bmp", "gif"];

const files = ref<string[]>([]);
const format = ref("png");
const width = ref(0);
const height = ref(0);
const quality = ref(85);
/** 转换进行中状态与已完成数量 */
const running = ref(false);
const done = ref(0);

function handleFiles(paths: string[]) {
  const valid = paths.filter(p => IMAGE_EXTS.includes(extOf(p).toLowerCase()));
  if (valid.length === 0) {
    message.warning(t("imageConvert.warnNoImage"));
    return;
  }
  files.value = valid;
}

function clearFiles() {
  files.value = [];
}

async function pickFiles() {
  const sel = await open({
    filters: [{ name: "Images", extensions: IMAGE_EXTS }],
    multiple: true,
  });
  if (sel) handleFiles(Array.isArray(sel) ? sel : [sel]);
}

function onDrop(e: DragEvent) {
  const paths = Array.from(e.dataTransfer?.files || []).map(f => (f as any).path);
  handleFiles(paths);
}

/** 输出路径：源目录（或默认输出目录）下同名换扩展名；同名格式加 _converted 避免覆盖源文件 */
function outputPathFor(f: string): string {
  const srcExt = extOf(f).toLowerCase();
  const base = f.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "output";
  const dir = defaultOutDir(f, settings.defaultOutDir);
  const sameFormat = srcExt === format.value || (srcExt === "jpeg" && format.value === "jpg");
  return `${dir}/${sameFormat ? `${base}_converted` : base}.${format.value}`;
}

async function run() {
  if (files.value.length === 0) return;
  running.value = true;
  done.value = 0;
  let ok = 0;
  let failed = 0;
  const outs: string[] = [];
  const w = Math.max(0, Math.round(width.value || 0));
  const h = Math.max(0, Math.round(height.value || 0));
  try {
    for (const f of files.value) {
      const out = outputPathFor(f);
      try {
        // 指定了尺寸走缩放（按输出扩展名保存）；否则纯格式转换
        if (w > 0 || h > 0) {
          await imageResize(f, out, w, h);
        } else {
          await imageConvert(f, out, quality.value);
        }
        ok++;
        outs.push(out);
      } catch (e: any) {
        failed++;
        message.error(t("imageConvert.fail", { name: f.split(/[\\/]/).pop(), err: e }));
      }
      done.value++;
    }
    await history.add({
      kind: "imageConvert",
      name: t("imageConvert.resultName", { n: ok }),
      inputs: files.value,
      outputs: outs,
      ok: ok > 0,
    });
    if (ok > 0) {
      resultText.value = t("imageConvert.success", { n: ok });
      resultOutputs.value = outs;
      void notifyDone(t("common.taskDone"), t("imageConvert.success", { n: ok }));
    }
  } finally {
    running.value = false;
  }
}

defineExpose({ handleDrop: handleFiles });
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
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.form { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.field { display: flex; flex-direction: column; gap: 8px; }
.field label { font-size: 13px; color: var(--text-sub); }
.op-options { display: flex; gap: 10px; flex-wrap: wrap; }
.size-row { display: flex; align-items: center; gap: 8px; }
.size-row > * { width: 130px; }
.size-x { color: var(--text-muted); width: auto !important; }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>
