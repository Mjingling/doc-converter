<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("merge.title") }}</h2>
      <p>{{ t("merge.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFiles">
      <div v-if="!mergeFiles.length" class="zone-empty">
        <div class="zone-icons">
          <NIcon :component="GitMergeOutline" :size="34" color="#e6494c" />
        </div>
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("merge.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="CloudUploadOutline" :size="20" color="#2080f0" />
        <span>{{ t("merge.added", { n: mergeFiles.length }) }}</span>
      </div>
    </div>

    <!-- 文件列表 -->
    <div v-if="mergeFiles.length" class="file-list">
      <div v-for="(f, i) in mergeFiles" :key="f + i" class="file-row">
        <span class="order">{{ i + 1 }}</span>
        <NIcon :component="DocumentTextOutline" :size="19" color="#e6494c" />
        <span class="fname" :title="f">{{ mergeNames[i] }}</span>
        <button class="remove-btn" @click="removeFile(i)">×</button>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ mergeFiles.length >= 2 ? t("merge.hintCount", { n: mergeFiles.length }) : t("merge.hintMin") }}</span>
      <button class="cta" :disabled="mergeFiles.length < 2 || running" @click="doMerge">
        <NIcon :component="GitMergeOutline" :size="17" />
        {{ running ? t("merge.running") : t("merge.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="running" indeterminate :label="t('merge.running')" />

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { CloudUploadOutline, DocumentTextOutline, GitMergeOutline } from "@vicons/ionicons5";
import { pdfMerge } from "../api";
import { notifyDone } from "../utils/notify";
import { maybeAutoOpenOutput } from "../utils/autoOpen";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";
import { defaultOutputPath } from "../utils/file";
import { usePanelTask } from "../composables/usePanelTask";

const { t } = useI18n();
const message = useMessage();
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");
const history = useHistoryStore();
const settings = useSettingsStore();

const mergeFiles = ref<string[]>([]);
const mergeNames = computed(() => mergeFiles.value.map((p) => p.split(/[\\/]/).pop() ?? p));

/** 执行状态：running + 进度条 */
const { running, run } = usePanelTask();

async function pickFiles() {
  const paths = await openDialog({
    multiple: true,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!paths) return;
  for (const p of paths) {
    if (!mergeFiles.value.includes(String(p))) mergeFiles.value.push(String(p));
  }
}

function removeFile(i: number) {
  mergeFiles.value.splice(i, 1);
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
function handleDrop(paths: string[]) {
  const pdfs = paths.filter((p) => /\.pdf$/i.test(p));
  if (!pdfs.length) {
    message.warning(t("merge.warnOnlyPdf"));
    return;
  }
  for (const p of pdfs) {
    if (!mergeFiles.value.includes(p)) mergeFiles.value.push(p);
  }
}
defineExpose({ handleDrop });

async function doMerge() {
  if (mergeFiles.value.length < 2) {
    message.warning(t("merge.warnMin"));
    return;
  }
  const outPath = defaultOutputPath(mergeFiles.value[0], "_merged", settings.defaultOutDir);
  await run(async () => {
    try {
      const out = await pdfMerge([...mergeFiles.value], outPath);
      const outName = out.split(/[\\/]/).pop() ?? out;
      resultText.value = t("merge.success", { name: outName });
      resultOutputs.value = [out];
      await history.add({ kind: "merge", name: outName, inputs: [...mergeFiles.value], outputs: [out], ok: true });
      void notifyDone(t("common.taskDone"), t("merge.success", { name: outName }));
      void maybeAutoOpenOutput(out);
    } catch (e) {
      message.error(t("merge.fail", { err: String(e) }));
      await history.add({
        kind: "merge",
        name: t("merge.cta"),
        inputs: [...mergeFiles.value],
        outputs: [],
        ok: false,
      });
    }
  });
}
</script>

<style scoped>
.panel {
  background: var(--bg-panel);
  border-radius: 14px;
  padding: 30px;
  box-shadow: 0 1px 3px var(--shadow);
}
.panel-head h2 {
  margin: 0;
  font-size: 22px;
  color: var(--text-main);
}
.panel-head p {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-muted);
}
.upload-zone {
  margin-top: 18px;
  border: 2px dashed var(--border-dash);
  border-radius: 14px;
  padding: 28px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
}
.upload-zone:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.zone-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}
.zone-icons {
  display: flex;
  align-items: center;
  gap: 14px;
}
.zone-main {
  margin: 0;
  font-size: 14px;
  color: var(--text-sub);
}
.zone-main .link {
  color: var(--accent);
  cursor: pointer;
}
.zone-sub {
  margin: 0;
  font-size: 12px;
  color: var(--text-faint);
}
.zone-filled {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 14px;
  color: var(--text-sub);
}
.file-list {
  margin-top: 14px;
}
.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 6px;
  border-top: 1px solid var(--border-soft);
}
.order {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--bg-tag);
  color: var(--text-muted);
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.fname {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--text-body);
}
.remove-btn {
  border: none;
  background: none;
  color: var(--text-muted);
  font-size: 18px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}
.remove-btn:hover { color: var(--red); }
.action-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border-soft);
}
.hint {
  font-size: 12px;
  color: var(--text-muted);
}
.cta {
  display: flex;
  align-items: center;
  gap: 8px;
  border: none;
  background: var(--cta-bg);
  color: var(--cta-text);
  font-size: 15px;
  font-weight: 600;
  padding: 11px 30px;
  border-radius: 8px;
  cursor: pointer;
  transition: opacity 0.15s;
}
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled {
  background: var(--cta-disabled);
  cursor: not-allowed;
}
</style>
