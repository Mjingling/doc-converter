<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("aiSummary.title") }}</h2>
      <p>{{ t("aiSummary.subtitle") }}</p>
    </div>

    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="!file" class="zone-empty">
        <NIcon :component="SparklesOutline" :size="34" color="#722ed1" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("aiSummary.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#722ed1" />
        <span class="fname" :title="file">{{ fileName }}</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <div v-if="file" class="form">
      <div class="field">
        <label>{{ t("aiSummary.formatLabel") }}</label>
        <div class="length-row">
          <button
            v-for="opt in formatOptions"
            :key="opt.value"
            class="length-btn"
            :class="{ active: format === opt.value }"
            @click="format = opt.value"
          >{{ t(opt.label) }}</button>
        </div>
      </div>
      <div class="field">
        <label>{{ t("aiSummary.lengthLabel") }}</label>
        <div class="length-row">
          <button
            v-for="opt in lengthOptions"
            :key="opt.value"
            class="length-btn"
            :class="{ active: length === opt.value }"
            @click="length = opt.value"
          >{{ t(opt.label) }}</button>
        </div>
      </div>
    </div>

    <div class="action-row">
      <span class="hint">{{ t("aiSummary.hint") }}</span>
      <button class="cta" :disabled="!file || loading" @click="run">
        <NIcon :component="SparklesOutline" :size="17" />
        {{ loading ? t("aiSummary.running") : t("aiSummary.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="loading" indeterminate :label="t('aiSummary.running')" />

    <!-- 原文预览：确认 AI 读取内容无误 -->
    <div v-if="previewText" class="preview-box">
      <div class="preview-head" @click="previewOpen = !previewOpen">
        <span class="preview-label">{{ t("aiSummary.previewLabel") }}</span>
        <NIcon :component="previewOpen ? ChevronUpOutline : ChevronDownOutline" :size="14" />
      </div>
      <div v-if="previewOpen" class="preview-body">{{ previewText }}</div>
    </div>

    <div v-if="summary" class="summary-box">
      <div class="summary-head">
        <span class="summary-label">{{ t("aiSummary.resultTitle") }}</span>
        <div class="summary-actions">
          <button class="mini-btn" @click="copySummary">
            <NIcon :component="CopyOutline" :size="15" />
            {{ copied ? t("aiSummary.copied") : t("aiSummary.copy") }}
          </button>
          <button class="mini-btn" :disabled="loading" @click="run">
            <NIcon :component="RefreshOutline" :size="15" />
            {{ t("aiSummary.regenerate") }}
          </button>
        </div>
      </div>
      <div class="summary-body">{{ summary }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon } from "naive-ui";
import { useI18n } from "vue-i18n";
import { SparklesOutline, DocumentTextOutline, CopyOutline, RefreshOutline, ChevronUpOutline, ChevronDownOutline } from "@vicons/ionicons5";
import { extOf } from "../utils/file";
import TaskProgress from "./TaskProgress.vue";
import { open } from "@tauri-apps/plugin-dialog";
import { extractText } from "../api";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";
import { chat } from "../ai";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();
const settings = useSettingsStore();

type SummaryLength = "short" | "standard" | "detailed";
/** 输出格式：摘要 / 要点列表 / 待办提取 / 会议纪要 / 脑图大纲 */
type OutputFormat = "summary" | "points" | "todos" | "minutes" | "outline";

const file = ref("");
const fileName = ref("");
const loading = ref(false);
const summary = ref("");
const copied = ref(false);
/** 提取到的原文预览（截断后）与展开状态 */
const previewText = ref("");
const previewOpen = ref(true);

const lengthOptions: { value: SummaryLength; label: string }[] = [
  { value: "short", label: "aiSummary.lengthShort" },
  { value: "standard", label: "aiSummary.lengthStandard" },
  { value: "detailed", label: "aiSummary.lengthDetailed" },
];
const length = ref<SummaryLength>("standard");

const formatOptions: { value: OutputFormat; label: string }[] = [
  { value: "summary", label: "aiSummary.formatSummary" },
  { value: "points", label: "aiSummary.formatPoints" },
  { value: "todos", label: "aiSummary.formatTodos" },
  { value: "minutes", label: "aiSummary.formatMinutes" },
  { value: "outline", label: "aiSummary.formatOutline" },
];
const format = ref<OutputFormat>("summary");

/** 摘要所需最大输入字符数（超出截断，控制 token 消耗） */
const MAX_INPUT_CHARS = 6000;


function handleFile(path: string) {
  const ext = extOf(path).toLowerCase();
  const supported = ["pdf", "docx", "txt", "md", "markdown", "csv", "json", "xml", "html", "htm", "log"];
  if (!supported.includes(ext)) {
    message.warning(t("aiSummary.warnUnsupported"));
    return;
  }
  file.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
  summary.value = "";
  previewText.value = "";
  previewOpen.value = true;
}

async function pickFile() {
  const sel = await open({ multiple: false });
  if (sel) handleFile(sel);
}

function onDrop(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile((f as any).path);
}

function clearFile() {
  file.value = "";
  fileName.value = "";
  summary.value = "";
  previewText.value = "";
}

async function run() {
  if (!file.value || loading.value) return;
  loading.value = true;
  summary.value = "";
  copied.value = false;
  try {
    // 1. 提取文档文本
    const raw = await extractText(file.value);
    if (!raw.trim()) {
      message.warning(t("aiSummary.noText"));
      return;
    }
    const text = raw.slice(0, MAX_INPUT_CHARS);
    previewText.value = text;

    // 2. 生成结果（chat 能力，auto 模式自动选择本地/云端引擎）
    const promptKey = `aiSummary.promptLength${length.value.charAt(0).toUpperCase() + length.value.slice(1)}`;
    const lengthPrompt = t(promptKey);
    // 输出格式 → 任务描述（summary 沿用经典摘要任务，其余格式用专用提示词）
    const taskPrompt = format.value === "summary"
      ? t("aiSummary.taskSummary")
      : t(`aiSummary.promptFormat${format.value.charAt(0).toUpperCase() + format.value.slice(1)}`);
    const result = await chat([
      { role: "system", content: t("aiSummary.promptSystem") },
      { role: "user", content: `${taskPrompt}，篇幅要求：${lengthPrompt}\n\n文档内容：\n${text}` },
    ]);
    summary.value = result.trim();

    history.add({
      kind: "aiSummary",
      name: t("aiSummary.resultName", { name: fileName.value }),
      inputs: [file.value],
      outputs: [],
      ok: true,
    });
  } catch (e: any) {
    history.add({
      kind: "aiSummary",
      name: t("aiSummary.resultName", { name: fileName.value }),
      inputs: [file.value],
      outputs: [],
      ok: false,
    });
    const msg = String(e || "");
    // 按模式分流提示文案：local 模式提示本地模型问题，cloud/auto 提示云端配置
    if (settings.ai.mode === "local") {
      message.error(t("aiSummary.fail", { err: "本地模型未下载或加载失败，请在设置 → AI 能力中下载本地模型" }));
    } else if (msg.includes("云端") || !settings.ai.cloud.baseUrl) {
      message.error(t("aiSummary.cloudUnset"));
    } else {
      message.error(t("aiSummary.fail", { err: msg }));
    }
  } finally {
    loading.value = false;
  }
}

async function copySummary() {
  try {
    await navigator.clipboard.writeText(summary.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    message.error(t("aiSummary.copyFail"));
  }
}

defineExpose({ handleDrop: (paths: string[]) => {
  if (paths.length > 0) handleFile(paths[0]);
}});
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
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.form { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.field { display: flex; flex-direction: column; gap: 8px; }
.field label { font-size: 13px; color: var(--text-sub); }
.length-row { display: flex; gap: 8px; }
.length-btn { border: 1px solid var(--border-soft); background: var(--bg-page); color: var(--text-sub); font-size: 13px; padding: 6px 16px; border-radius: 20px; cursor: pointer; transition: all 0.15s; }
.length-btn:hover { border-color: var(--accent); color: var(--accent); }
.length-btn.active { background: var(--accent); border-color: var(--accent); color: var(--cta-text); font-weight: 600; }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
.preview-box { margin-top: 16px; border: 1px solid var(--border-soft); border-radius: 10px; overflow: hidden; }
.preview-head { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; background: var(--bg-page); border-bottom: 1px solid var(--border-soft); cursor: pointer; color: var(--text-sub); }
.preview-head:hover { color: var(--accent); }
.preview-label { font-size: 13px; font-weight: 600; }
.preview-body { padding: 14px; font-size: 12px; line-height: 1.7; color: var(--text-muted); white-space: pre-wrap; word-break: break-word; max-height: 200px; overflow-y: auto; }
.summary-box { margin-top: 16px; border: 1px solid var(--border-soft); border-radius: 10px; overflow: hidden; }
.summary-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 10px 14px; background: var(--bg-page); border-bottom: 1px solid var(--border-soft); }
.summary-label { font-size: 13px; font-weight: 600; color: var(--accent); }
.summary-actions { display: flex; gap: 8px; }
.mini-btn { display: flex; align-items: center; gap: 4px; border: 1px solid var(--border-soft); background: var(--bg-panel); color: var(--text-sub); font-size: 12px; padding: 4px 10px; border-radius: 6px; cursor: pointer; transition: all 0.15s; }
.mini-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.mini-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.summary-body { padding: 14px; font-size: 13px; line-height: 1.7; color: var(--text-body); white-space: pre-wrap; word-break: break-word; max-height: 420px; overflow-y: auto; }
</style>
