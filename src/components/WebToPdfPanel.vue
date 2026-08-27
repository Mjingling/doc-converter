<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("webToPdf.title") }}</h2>
      <p>{{ t("webToPdf.subtitle") }}</p>
    </div>

    <div class="form">
      <div class="field">
        <label>{{ t("webToPdf.urlLabel") }}</label>
        <NInput v-model:value="url" :placeholder="t('webToPdf.urlPlaceholder')" @keyup.enter="run" />
      </div>
      <div class="field">
        <label>{{ t("webToPdf.engineLabel") }}</label>
        <div class="engine-row">
          <NRadioGroup v-model:value="engine">
            <NRadioButton value="rendered">{{ t("webToPdf.engineRendered") }}</NRadioButton>
            <NRadioButton value="light">{{ t("webToPdf.engineLight") }}</NRadioButton>
          </NRadioGroup>
          <span class="engine-hint">{{ t("webToPdf.engineHint." + engine) }}</span>
        </div>
      </div>
    </div>

    <div class="action-row">
      <span class="hint">{{ t("webToPdf.hint") }}</span>
      <button class="cta" :disabled="!url || loading" @click="run">
        <NIcon :component="GlobeOutline" :size="17" />
        {{ loading ? t("webToPdf.running") : t("webToPdf.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="loading" indeterminate :label="t('webToPdf.running')" />

    <div v-if="resultPath" class="results">
      <p class="result-title">{{ t("webToPdf.success", { name: resultName }) }}</p>
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
import { useMessage, NIcon, NInput, NButton, NRadioGroup, NRadioButton } from "naive-ui";
import { useI18n } from "vue-i18n";
import { GlobeOutline } from "@vicons/ionicons5";
import { webpageToPdf, webpageToPdfRendered, openPath } from "../api";
import { dirOf, defaultOutDir } from "../utils/file";
import { notifyDone } from "../utils/notify";
import TaskProgress from "./TaskProgress.vue";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();
const settings = useSettingsStore();

const url = ref("");
const engine = ref<"rendered" | "light">("rendered");
const resultPath = ref("");
const resultName = ref("");
const loading = ref(false);

/** 规范化 URL：自动补全协议并校验格式，非法时返回 null */
function normalizeUrl(raw: string): string | null {
  let u = raw.trim();
  if (!u) return null;
  if (!/^https?:\/\//i.test(u)) u = "https://" + u;
  try {
    const parsed = new URL(u);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return null;
    return parsed.href;
  } catch {
    return null;
  }
}

async function run() {
  if (!url.value) return;
  const target = normalizeUrl(url.value);
  if (!target) {
    message.warning(t("webToPdf.warnInvalidUrl"));
    return;
  }
  loading.value = true;
  try {
    const dir = defaultOutDir("", settings.defaultOutDir);
    const out =
      engine.value === "rendered"
        ? await webpageToPdfRendered(target, `${dir}/webpage.pdf`)
        : await webpageToPdf(target, `${dir}/webpage.pdf`);
    resultPath.value = out;
    resultName.value = out.split(/[/\\]/).pop() || out;
    history.add({ kind: "webToPdf", name: url.value, inputs: [url.value], outputs: [out], ok: true });
    message.success(t("webToPdf.success", { name: resultName.value }));
    void notifyDone(t("common.taskDone"), t("webToPdf.success", { name: resultName.value }));
  } catch (e: any) {
    history.add({ kind: "webToPdf", name: url.value, inputs: [url.value], outputs: [], ok: false });
    message.error(t("webToPdf.fail", { err: e }));
  } finally {
    loading.value = false;
  }
}
</script>

<style scoped>
.panel { background: var(--bg-panel); border-radius: 14px; padding: 30px; box-shadow: 0 1px 3px var(--shadow); }
.panel-head h2 { margin: 0; font-size: 22px; color: var(--text-main); }
.panel-head p { margin: 6px 0 0; font-size: 13px; color: var(--text-muted); }
.form { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.field { display: flex; flex-direction: column; gap: 8px; }
.field label { font-size: 13px; color: var(--text-sub); }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.engine-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.engine-hint { font-size: 12px; color: var(--text-muted); }
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
