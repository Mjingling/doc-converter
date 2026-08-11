<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("watermark.title") }}</h2>
      <p>{{ t("watermark.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile">
      <div v-if="!pdfFile" class="zone-empty">
        <NIcon :component="WaterOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("watermark.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="pdfFile">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
      </div>
    </div>

    <!-- 水印配置 -->
    <div v-if="pdfFile" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("watermark.textLabel") }}</label>
        <NInput
          v-model:value="text"
          :placeholder="t('watermark.textPlaceholder')"
          maxlength="30"
          clearable
          size="large"
        />
      </div>
      <div class="config-row">
        <label class="config-label">{{ t("watermark.opacityLabel") }}</label>
        <div class="opacity-row">
          <NSlider v-model:value="opacity" :min="0.05" :max="1" :step="0.05" style="flex: 1" />
          <span class="opacity-value">{{ Math.round(opacity * 100) }}%</span>
        </div>
      </div>
    </div>

    <!-- 输出目录 -->
    <div v-if="pdfFile" class="out-dir-field">
      <div class="config-label out-dir-label">{{ t("watermark.outDirLabel") }}</div>
      <div class="out-dir" @click="pickDir">
        <span class="out-dir-text">{{ outDir || t("convert.outDirDefault") }}</span>
        <span class="out-dir-btn">{{ t("settings.choose") }}</span>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("watermark.hint") }}</span>
      <button class="cta" :disabled="!pdfFile" @click="doWatermark">
        <NIcon :component="WaterOutline" :size="17" />
        {{ t("watermark.cta") }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInput, NSlider, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { DocumentTextOutline, WaterOutline } from "@vicons/ionicons5";
import { pdfWatermark } from "../api";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const history = useHistoryStore();

const pdfFile = ref("");
const fileName = computed(() => pdfFile.value.split(/[\\/]/).pop() ?? pdfFile.value);
/** 水印文字（默认按语言给一个示例） */
const text = ref(t("watermark.defaultText"));
/** 不透明度 5%~100% */
const opacity = ref(0.2);
// 输出目录：初始化用设置中的默认目录；手动选择后记住上次目录
const outDir = ref(settings.defaultOutDir);
let lastChosenDir = "";

async function pickFile() {
  const p = await openDialog({ filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!p) return;
  pdfFile.value = String(p);
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
function handleDrop(paths: string[]) {
  const pdf = paths.find((p) => /\.pdf$/i.test(p));
  if (!pdf) {
    message.warning(t("watermark.warnOnlyPdf"));
    return;
  }
  pdfFile.value = pdf;
}
defineExpose({ handleDrop });

async function pickDir() {
  const d = await openDialog({ directory: true, title: t("watermark.pickDirTitle") });
  if (d) {
    outDir.value = String(d);
    lastChosenDir = String(d);
  }
}

async function doWatermark() {
  if (!pdfFile.value) {
    message.warning(t("watermark.warnNoFile"));
    return;
  }
  const content = text.value.trim();
  if (!content) {
    message.warning(t("watermark.warnNoText"));
    return;
  }
  // 输出目录：上次手动选择的目录优先；没有则输出到源文件所在目录
  let dir = outDir.value || lastChosenDir;
  if (!dir) {
    const i = Math.max(pdfFile.value.lastIndexOf("/"), pdfFile.value.lastIndexOf("\\"));
    dir = pdfFile.value.slice(0, i);
  }
  const stem = (pdfFile.value.split(/[\\/]/).pop() || "output").replace(/\.pdf$/i, "");
  const outPath = `${dir}/${stem}_watermarked.pdf`;
  try {
    const out = await pdfWatermark(pdfFile.value, outPath, content, opacity.value);
    const outName = out.split(/[\\/]/).pop() ?? out;
    message.success(t("watermark.success", { name: outName }), { duration: 4000 });
    await history.add({
      kind: "watermark",
      name: outName,
      inputs: [pdfFile.value],
      outputs: [out],
      ok: true,
    });
  } catch (e) {
    message.error(t("watermark.fail", { err: String(e) }));
    await history.add({
      kind: "watermark",
      name: fileName.value,
      inputs: [pdfFile.value],
      outputs: [],
      ok: false,
    });
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
/* 配置区 */
.config {
  margin-top: 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.config-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.config-label {
  font-size: 13px;
  color: var(--text-sub);
}
.opacity-row {
  display: flex;
  align-items: center;
  gap: 14px;
}
.opacity-value {
  font-size: 13px;
  color: var(--text-main);
  width: 44px;
  text-align: right;
}
.out-dir-field {
  margin-top: 18px;
}
.out-dir-label {
  margin-bottom: 8px;
}
.out-dir {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  padding: 8px 12px;
  cursor: pointer;
  max-width: 420px;
}
.out-dir:hover {
  border-color: var(--accent);
}
.out-dir-text {
  font-size: 13px;
  color: var(--text-sub);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.out-dir-btn {
  font-size: 12px;
  color: var(--accent);
  flex-shrink: 0;
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
