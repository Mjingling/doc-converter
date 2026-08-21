<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("metadata.title") }}</h2>
      <p>{{ t("metadata.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="!filePath" class="zone-empty">
        <NIcon :component="InformationCircleOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("metadata.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="filePath">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <!-- 元数据表单 -->
    <div v-if="filePath" class="form">
      <div class="field">
        <label>{{ t("metadata.titleLabel") }}</label>
        <NInput v-model:value="title" :placeholder="t('metadata.titlePlaceholder')" clearable />
      </div>
      <div class="field">
        <label>{{ t("metadata.authorLabel") }}</label>
        <NInput v-model:value="author" :placeholder="t('metadata.authorPlaceholder')" clearable />
      </div>
      <div class="field">
        <label>{{ t("metadata.subjectLabel") }}</label>
        <NInput v-model:value="subject" :placeholder="t('metadata.subjectPlaceholder')" clearable />
      </div>
      <div class="field">
        <label>{{ t("metadata.keywordsLabel") }}</label>
        <NInput v-model:value="keywords" :placeholder="t('metadata.keywordsPlaceholder')" clearable />
      </div>
    </div>

    <!-- 输出目录 -->
    <div v-if="filePath" class="out-dir" @click="pickDir">
      <span class="out-dir-label">{{ t("metadata.outDirLabel") }}</span>
      <span class="out-dir-text">{{ outDir || t("metadata.outDirPlaceholder") }}</span>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("metadata.hint") }}</span>
      <button class="cta" :disabled="!filePath" @click="run">
        <NIcon :component="InformationCircleOutline" :size="17" />
        {{ t("metadata.cta") }}
      </button>
    </div>

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon, NInput } from "naive-ui";
import { useI18n } from "vue-i18n";
import { DocumentTextOutline, InformationCircleOutline } from "@vicons/ionicons5";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { pdfMetadata } from "../api";
import ResultBar from "./ResultBar.vue";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";
import { dirOf } from "../utils/file";

const { t } = useI18n();
const message = useMessage();
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");
const history = useHistoryStore();
const settings = useSettingsStore();

const filePath = ref("");
const fileName = ref("");
const title = ref("");
const author = ref("");
const subject = ref("");
const keywords = ref("");
// 输出目录：初始化用设置中的默认目录；手动选择后记住上次目录
const outDir = ref(settings.defaultOutDir);
let lastChosenDir = "";

async function pickDir() {
  const d = await openDialog({ directory: true, title: t("metadata.pickDirTitle") });
  if (d) {
    outDir.value = String(d);
    lastChosenDir = String(d);
  }
}

function handleFile(path: string) {
  if (!path.toLowerCase().endsWith(".pdf")) {
    message.warning(t("metadata.warnOnlyPdf"));
    return;
  }
  filePath.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
}

function clearFile() {
  filePath.value = "";
  fileName.value = "";
  title.value = "";
  author.value = "";
  subject.value = "";
  keywords.value = "";
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

async function run() {
  if (!filePath.value) return;
  // 输出目录：上次手动选择的目录优先；没有则输出到源文件所在目录
  let dir = outDir.value || lastChosenDir;
  if (!dir) dir = dirOf(filePath.value);
  const stem = fileName.value.replace(/\.pdf$/i, "");
  const out = `${dir}/${stem}_metadata.pdf`;
  try {
    await pdfMetadata(filePath.value, out, title.value || null, author.value || null, subject.value || null, keywords.value || null);
    history.add({ kind: "metadata", name: fileName.value, inputs: [filePath.value], outputs: [out], ok: true });
    resultText.value = t("metadata.success", { name: fileName.value });
    resultOutputs.value = [out];
  } catch (e: any) {
    history.add({ kind: "metadata", name: fileName.value, inputs: [filePath.value], outputs: [], ok: false });
    message.error(t("metadata.fail", { err: e }));
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
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
/* 输出目录选择器 */
.out-dir {
  margin-top: 18px;
  padding: 10px 14px;
  border: 1px dashed var(--border-dash);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.out-dir:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.out-dir-label {
  font-size: 13px;
  color: var(--text-sub);
}
.out-dir-text {
  font-size: 13px;
  color: var(--text-muted);
}
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>