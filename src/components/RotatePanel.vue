<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("rotate.title") }}</h2>
      <p>{{ t("rotate.subtitle") }}</p>
    </div>

    <!-- 工具切换：旋转 / 加页码 -->
    <div class="mode-tabs">
      <button
        v-for="m in modes"
        :key="m"
        class="mode-tab"
        :class="{ active: mode === m }"
        @click="mode = m"
      >
        {{ t(`rotate.tab${m === "rotate" ? "Rotate" : "Pages"}`) }}
      </button>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile">
      <div v-if="!pdfFile" class="zone-empty">
        <NIcon :component="mode === 'rotate' ? RefreshOutline : ListOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("rotate.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="pdfFile">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
      </div>
    </div>

    <!-- 旋转配置 -->
    <div v-if="pdfFile && mode === 'rotate'" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("rotate.angleLabel") }}</label>
        <NRadioGroup v-model:value="angle">
          <div class="angle-options">
            <NRadioButton v-for="a in [90, 180, 270]" :key="a" :value="a" :label="`${a}°`" />
          </div>
        </NRadioGroup>
      </div>
    </div>

    <!-- 页码配置 -->
    <div v-if="pdfFile && mode === 'pages'" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("rotate.styleLabel") }}</label>
        <NRadioGroup v-model:value="style">
          <div class="angle-options">
            <NRadioButton value="page" :label="t('rotate.stylePage')" />
            <NRadioButton value="pageOf" :label="t('rotate.stylePageOf')" />
          </div>
        </NRadioGroup>
      </div>
    </div>

    <!-- 输出目录 -->
    <div v-if="pdfFile" class="out-dir-field">
      <div class="config-label out-dir-label">{{ t("rotate.outDirLabel") }}</div>
      <div class="out-dir" @click="pickDir">
        <span class="out-dir-text">{{ outDir || t("convert.outDirDefault") }}</span>
        <span class="out-dir-btn">{{ t("settings.choose") }}</span>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t(mode === "rotate" ? "rotate.hintRotate" : "rotate.hintPages") }}</span>
      <button class="cta" :disabled="!pdfFile" @click="doWork">
        <NIcon :component="mode === 'rotate' ? RefreshOutline : ListOutline" :size="17" />
        {{ t(mode === "rotate" ? "rotate.ctaRotate" : "rotate.ctaPages") }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NRadioButton, NRadioGroup, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { DocumentTextOutline, ListOutline, RefreshOutline } from "@vicons/ionicons5";
import { pdfPageNumbers, pdfRotate } from "../api";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const history = useHistoryStore();

type Mode = "rotate" | "pages";
const modes: Mode[] = ["rotate", "pages"];
const mode = ref<Mode>("rotate");

const pdfFile = ref("");
const fileName = computed(() => pdfFile.value.split(/[\\/]/).pop() ?? pdfFile.value);
/** 旋转角度（90 / 180 / 270） */
const angle = ref(90);
/** 页码样式：仅页码 / 页码+总页数 */
const style = ref<"page" | "pageOf">("page");
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
    message.warning(t("rotate.warnOnlyPdf"));
    return;
  }
  pdfFile.value = pdf;
}
defineExpose({ handleDrop });

async function pickDir() {
  const d = await openDialog({ directory: true, title: t("rotate.pickDirTitle") });
  if (d) {
    outDir.value = String(d);
    lastChosenDir = String(d);
  }
}

async function doWork() {
  if (!pdfFile.value) {
    message.warning(t("rotate.warnNoFile"));
    return;
  }
  const suffix = mode.value === "rotate" ? "_rotated" : "_numbered";
  // 输出目录：上次手动选择的目录优先；没有则输出到源文件所在目录
  let dir = outDir.value || lastChosenDir;
  if (!dir) {
    const i = Math.max(pdfFile.value.lastIndexOf("/"), pdfFile.value.lastIndexOf("\\"));
    dir = pdfFile.value.slice(0, i);
  }
  const stem = (pdfFile.value.split(/[\\/]/).pop() || "output").replace(/\.pdf$/i, "");
  const outPath = `${dir}/${stem}${suffix}.pdf`;
  const kind = mode.value === "rotate" ? "rotate" : "pages";
  try {
    const out =
      mode.value === "rotate"
        ? await pdfRotate(pdfFile.value, outPath, angle.value)
        : await pdfPageNumbers(pdfFile.value, outPath, style.value);
    const outName = out.split(/[\\/]/).pop() ?? out;
    message.success(t("rotate.success", { name: outName }), { duration: 4000 });
    await history.add({ kind, name: outName, inputs: [pdfFile.value], outputs: [out], ok: true });
  } catch (e) {
    message.error(t("rotate.fail", { err: String(e) }));
    await history.add({ kind, name: fileName.value, inputs: [pdfFile.value], outputs: [], ok: false });
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
.mode-tabs {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}
.mode-tab {
  border: 1px solid var(--border-strong);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 13px;
  padding: 6px 18px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.mode-tab.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}
.upload-zone {
  margin-top: 16px;
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
.angle-options {
  display: flex;
  gap: 8px;
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
