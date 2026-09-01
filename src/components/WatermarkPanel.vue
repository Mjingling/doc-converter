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
      <div class="config-row">
        <label class="config-label">{{ t("watermark.colorLabel") }}</label>
        <NColorPicker v-model:value="color" :show-alpha="false" style="width: 100%" />
      </div>
      <div class="config-row">
        <label class="config-label">{{ t("watermark.sizeLabel") }}</label>
        <div class="opacity-row">
          <NSlider v-model:value="fontSize" :min="10" :max="72" :step="1" style="flex: 1" />
          <span class="opacity-value">{{ fontSize }}</span>
        </div>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("watermark.hint") }}</span>
      <button class="cta" :disabled="!pdfFile || running" @click="doWatermark">
        <NIcon :component="WaterOutline" :size="17" />
        {{ running ? t("watermark.running") : t("watermark.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="running" indeterminate :label="t('watermark.running')" />

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInput, NSlider, NColorPicker, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { DocumentTextOutline, WaterOutline } from "@vicons/ionicons5";
import { pdfWatermark } from "../api";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";
import { defaultOutputPath } from "../utils/file";
import { usePanelTask } from "../composables/usePanelTask";

const { t } = useI18n();
const message = useMessage();
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");
const settings = useSettingsStore();
const history = useHistoryStore();

const pdfFile = ref("");
const fileName = computed(() => pdfFile.value.split(/[\\/]/).pop() ?? pdfFile.value);
/** 水印文字（默认按语言给一个示例） */
const text = ref(t("watermark.defaultText"));
/** 不透明度 5%~100% */
const opacity = ref(0.2);
/** 水印颜色（默认灰色，贴近办公场景） */
const color = ref<string | null>("#808080");
/** 水印字号 */
const fontSize = ref(26);

/** 执行状态：running + 进度条 */
const { running, run } = usePanelTask({ panelId: "watermark", label: t("watermark.title") });
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
  const outPath = defaultOutputPath(pdfFile.value, "_watermarked", settings.defaultOutDir);
  // 颜色：hex → RGB 0~255
  const hex = (color.value ?? "#808080").replace("#", "");
  const rgb: [number, number, number] = [
    parseInt(hex.slice(0, 2), 16) || 128,
    parseInt(hex.slice(2, 4), 16) || 128,
    parseInt(hex.slice(4, 6), 16) || 128,
  ];
  await run(async () => {
    try {
      const out = await pdfWatermark(pdfFile.value, outPath, content, opacity.value, rgb, fontSize.value);
      const outName = out.split(/[\\/]/).pop() ?? out;
      resultText.value = t("watermark.success", { name: outName });
      resultOutputs.value = [out];
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
