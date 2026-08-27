<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("imageCompress.title") }}</h2>
      <p>{{ t("imageCompress.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFiles" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="files.length === 0" class="zone-empty">
        <NIcon :component="ContractOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("imageCompress.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="CloudUploadOutline" :size="20" color="#2080f0" />
        <span>{{ t("imageCompress.added", { n: files.length }) }}</span>
        <button class="clear-btn" @click.stop="clearFiles">&times;</button>
      </div>
    </div>

    <div v-if="files.length > 0" class="form">
      <div class="field">
        <label>{{ t("imageCompress.qualityLabel") }}: {{ quality }}</label>
        <NSlider v-model:value="quality" :min="10" :max="100" :step="5" />
        <span class="hint">{{ t("imageCompress.qualityHint") }}</span>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("imageCompress.dropSub") }}</span>
      <button class="cta" :disabled="files.length === 0 || running" @click="run">
        <NIcon :component="ContractOutline" :size="17" />
        {{ running ? t("imageCompress.running", { done, total: files.length }) : t("imageCompress.cta") }}
      </button>
    </div>

    <!-- 执行进度：多文件真实百分比 -->
    <TaskProgress
      :running="running"
      :progress="Math.round((done / files.length) * 100)"
      :label="t('imageCompress.running', { done, total: files.length })"
    />

    <!-- 结果栏：打开文件 / 打开目录（压缩为原地覆盖，输出即源文件） -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon, NSlider } from "naive-ui";
import { useI18n } from "vue-i18n";
import { CloudUploadOutline, ContractOutline } from "@vicons/ionicons5";
import { extOf } from "../utils/file";
import { open } from "@tauri-apps/plugin-dialog";
import { imageCompress } from "../api";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";

const { t } = useI18n();
const message = useMessage();
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");

const files = ref<string[]>([]);
const quality = ref(75);
/** 压缩进行中状态与已完成数量 */
const running = ref(false);
const done = ref(0);

function handleFiles(paths: string[]) {
  const valid = paths.filter(p => {
    const ext = extOf(p).toLowerCase();
    return ext === "jpg" || ext === "jpeg" || ext === "png";
  });
  if (valid.length === 0) {
    message.warning(t("imageCompress.warnNoImage"));
    return;
  }
  files.value = valid;
}

function clearFiles() {
  files.value = [];
}

async function pickFiles() {
  const sel = await open({
    filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png"] }],
    multiple: true,
  });
  if (sel) handleFiles(Array.isArray(sel) ? sel : [sel]);
}

function onDrop(e: DragEvent) {
  const paths = Array.from(e.dataTransfer?.files || []).map(f => (f as any).path);
  handleFiles(paths);
}

async function run() {
  if (files.value.length === 0) return;
  running.value = true;
  done.value = 0;
  let ok = 0;
  let skipped = 0;
  const outs: string[] = [];
  try {
    for (const f of files.value) {
      try {
        if (await imageCompress(f, quality.value)) {
          ok++;
          outs.push(f); // 原地覆盖，输出即源文件
        } else {
          skipped++; // 重压无收益，保留原文件
        }
      } catch (e: any) {
        message.error(t("imageCompress.fail", { err: e }));
      }
      done.value++;
    }
    if (ok > 0) {
      resultText.value = skipped > 0
        ? t("imageCompress.successSkipped", { n: ok, m: skipped })
        : t("imageCompress.success", { n: ok });
      resultOutputs.value = outs;
    } else if (skipped > 0) {
      resultText.value = t("imageCompress.allOptimal", { n: skipped });
      resultOutputs.value = files.value; // 文件未改动，仍提供打开入口
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
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>