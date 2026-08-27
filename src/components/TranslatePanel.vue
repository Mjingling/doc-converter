<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("translate.title") }}</h2>
      <p>{{ t("translate.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="!file" class="zone-empty">
        <NIcon :component="LanguageOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("translate.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#2080f0" />
        <span class="fname" :title="file">{{ fileName }}</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <!-- 目标语言 -->
    <div v-if="file" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("translate.langLabel") }}</label>
        <NRadioGroup v-model:value="lang">
          <div class="op-options">
            <NRadioButton v-for="l in langs" :key="l.value" :value="l.value" :label="t(l.label)" />
          </div>
        </NRadioGroup>
      </div>
    </div>

    <!-- 云端配置提示 -->
    <div v-if="!cloudReady" class="cloud-warn">
      <NIcon :component="CloudOfflineOutline" :size="16" />
      <span>{{ t("aiAssistant.needCloud") }}</span>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("translate.outHint") }}</span>
      <button class="cta" :disabled="!file || running || !cloudReady" @click="run">
        <NIcon :component="LanguageOutline" :size="17" />
        {{ running ? t("translate.running", { done, total }) : t("translate.cta") }}
      </button>
    </div>

    <!-- 执行进度：按翻译块计数 -->
    <TaskProgress
      :running="running"
      :progress="total > 0 ? Math.round((done / total) * 100) : 0"
      :label="t('translate.running', { done, total })"
    />

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NRadioButton, NRadioGroup, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { CloudOfflineOutline, DocumentTextOutline, LanguageOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { extractText, saveTextFile } from "../api";
import { chat } from "../ai";
import { extOf, defaultOutDir } from "../utils/file";
import { splitForTranslate } from "../utils/translate";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";
import { notifyDone } from "../utils/notify";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const history = useHistoryStore();

/** 云端配置就绪（翻译依赖云端 chat） */
const cloudReady = computed(() => !!(settings.ai.cloud.baseUrl && settings.ai.cloud.apiKey));

/** 支持翻译的文档扩展名 */
const SUPPORTED_EXTS = ["pdf", "docx", "txt", "md", "markdown"];

const langs = [
  { value: "zh", label: "translate.langZh" },
  { value: "en", label: "translate.langEn" },
  { value: "ja", label: "translate.langJa" },
  { value: "ko", label: "translate.langKo" },
  { value: "fr", label: "translate.langFr" },
  { value: "de", label: "translate.langDe" },
];

const file = ref("");
const fileName = ref("");
const lang = ref("en");
const running = ref(false);
const done = ref(0);
const total = ref(0);
const resultOutputs = ref<string[]>([]);
const resultText = ref("");

function handleFile(path: string) {
  if (!SUPPORTED_EXTS.includes(extOf(path).toLowerCase())) {
    message.warning(t("translate.warnUnsupported"));
    return;
  }
  file.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
  resultText.value = "";
  resultOutputs.value = [];
}

async function pickFile() {
  const sel = await open({ multiple: false });
  if (sel) handleFile(String(sel));
}

function onDrop(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile((f as any).path);
}

function clearFile() {
  file.value = "";
  fileName.value = "";
}

/** 输出路径：源目录（或默认输出目录）下同名 _translated.md */
function outputPath(): string {
  const base = file.value.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "output";
  return `${defaultOutDir(file.value, settings.defaultOutDir)}/${base}_translated.md`;
}

async function run() {
  if (!file.value || running.value) return;
  running.value = true;
  done.value = 0;
  total.value = 0;
  try {
    const raw = await extractText(file.value);
    if (!raw.trim()) {
      message.warning(t("translate.noText"));
      return;
    }
    const chunks = splitForTranslate(raw);
    total.value = chunks.length;
    const langName = t(`translate.langName_${lang.value}`);
    const parts: string[] = [];
    for (const chunk of chunks) {
      const reply = await chat([
        { role: "system", content: t("translate.promptSystem") },
        { role: "user", content: `${t("translate.promptUser", { lang: langName })}\n\n${chunk}` },
      ]);
      // 双语输出：原文块与译文块交替，分隔线间隔
      parts.push(`${chunk}\n\n---\n\n${reply.trim()}`);
      done.value++;
    }
    const out = outputPath();
    await saveTextFile(out, parts.join("\n\n---\n\n") + "\n");
    await history.add({
      kind: "translate",
      name: fileName.value,
      inputs: [file.value],
      outputs: [out],
      ok: true,
    });
    resultText.value = t("translate.success");
    resultOutputs.value = [out];
    void notifyDone(t("common.taskDone"), t("translate.success"));
  } catch (e: any) {
    message.error(t("translate.fail", { err: String(e) }));
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
.fname { max-width: 400px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.config { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.config-row { display: flex; flex-direction: column; gap: 8px; }
.config-label { font-size: 13px; color: var(--text-sub); }
.op-options { display: flex; gap: 10px; flex-wrap: wrap; }
.cloud-warn { display: flex; align-items: center; gap: 8px; margin-top: 14px; font-size: 12px; color: var(--orange); background: var(--orange-soft); border-radius: 10px; padding: 10px 14px; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>
