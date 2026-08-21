<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("compress.title") }}</h2>
      <p>{{ t("compress.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile">
      <div v-if="!compressFile" class="zone-empty">
        <div class="zone-icons">
          <NIcon :component="ArchiveOutline" :size="34" color="#e6494c" />
        </div>
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("compress.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="compressFile">{{ compressFileName }}</span>
        <span class="size-tag">PDF</span>
      </div>
    </div>

    <!-- 说明 -->
    <div v-if="compressFile" class="note">
      {{ t("compress.note") }}
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("compress.hint") }}</span>
      <button class="cta" :disabled="!compressFile || loading" @click="doCompress">
        <NIcon :component="ArchiveOutline" :size="17" />
        {{ loading ? t("compress.compressing") : t("compress.cta") }}
      </button>
    </div>

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ArchiveOutline, DocumentTextOutline } from "@vicons/ionicons5";
import { pdfCompress } from "../api";
import ResultBar from "./ResultBar.vue";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";
import { defaultOutputPath } from "../utils/file";
import { triggerOutputDirPrompt } from "../composables/useOutputDirPrompt";

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const history = useHistoryStore();

const compressFile = ref("");
const compressFileName = computed(() => compressFile.value.split(/[\\/]/).pop() ?? compressFile.value);
const loading = ref(false);
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");
async function pickFile() {
  const p = await openDialog({
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!p) return;
  compressFile.value = String(p);
  triggerOutputDirPrompt(String(p));
  resultOutputs.value = [];
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
function handleDrop(paths: string[]) {
  const pdf = paths.find((p) => /[.]pdf$/i.test(p));
  if (!pdf) {
    message.warning(t("compress.warnOnlyPdf"));
    return;
  }
  compressFile.value = pdf;
  triggerOutputDirPrompt(pdf);
}
defineExpose({ handleDrop });

async function doCompress() {
  if (!compressFile.value) {
    message.warning(t("compress.warnNoFile"));
    return;
  }
  const outPath = defaultOutputPath(compressFile.value, "_compressed", settings.defaultOutDir);
  loading.value = true;
  try {
    const out = await pdfCompress(compressFile.value, outPath);
    const outName = out.split(/[\\/]/).pop() ?? out;
    resultText.value = t("compress.success", { name: outName });
    resultOutputs.value = [out];
    await history.add({ kind: "compress", name: outName, inputs: [compressFile.value], outputs: [out], ok: true });
  } catch (e) {
    message.error(t("compress.fail", { err: String(e) }));
    await history.add({
      kind: "compress",
      name: compressFileName.value,
      inputs: [compressFile.value],
      outputs: [],
      ok: false,
    });
  } finally {
    loading.value = false;
  }
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
  gap: 10px;
  font-size: 14px;
  color: var(--text-sub);
}
.fname {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--text-body);
}
.size-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 8px;
  color: var(--text-muted);
  background: var(--bg-tag);
}
.note {
  margin-top: 14px;
  font-size: 12px;
  color: var(--text-muted);
  background: var(--bg-input);
  border-radius: 8px;
  padding: 10px 14px;
}
.action-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 18px;
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
