<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("organize.title") }}</h2>
      <p>{{ t("organize.subtitle") }}</p>
    </div>

    <!-- 工具切换：提取页面 / 删除页面 -->
    <div class="mode-tabs">
      <button
        v-for="m in modes"
        :key="m"
        class="mode-tab"
        :class="{ active: mode === m }"
        @click="mode = m"
      >
        {{ t(`organize.tab${m === "extract" ? "Extract" : "Delete"}`) }}
      </button>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile">
      <div v-if="!pdfFile" class="zone-empty">
        <NIcon :component="mode === 'extract' ? DocumentOutline : TrashOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("organize.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="pdfFile">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
        <span v-if="pageCount > 0" class="pages-tag">{{ t("organize.pagesOf", { n: pageCount }) }}</span>
      </div>
    </div>

    <!-- 页面配置 -->
    <div v-if="pdfFile" class="config">
      <div class="config-row">
        <label class="config-label">
          {{ mode === "extract" ? t("organize.extractLabel") : t("organize.deleteLabel") }}
        </label>
        <NInput
          v-model:value="spec"
          :placeholder="mode === 'extract' ? t('organize.extractPlaceholder') : t('organize.deletePlaceholder')"
          @keyup.enter="doWork"
        />
        <span class="config-hint">
          {{ mode === "extract" ? t("organize.extractHint") : t("organize.deleteHint") }}
        </span>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("organize.hint") }}</span>
      <button class="cta" :disabled="!pdfFile" @click="doWork">
        <NIcon :component="mode === 'extract' ? DocumentOutline : TrashOutline" :size="17" />
        {{ t(mode === "extract" ? "organize.ctaExtract" : "organize.ctaDelete") }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInput, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { DocumentOutline, DocumentTextOutline, TrashOutline } from "@vicons/ionicons5";
import { getPdfPageCount, pdfDeletePages, pdfExtractPages } from "../api";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();

type Mode = "extract" | "delete";
const modes: Mode[] = ["extract", "delete"];
const mode = ref<Mode>("extract");

const pdfFile = ref("");
const fileName = computed(() => pdfFile.value.split(/[\\/]/).pop() ?? pdfFile.value);
/** 文档总页数（选中文件后获取，用于前置校验） */
const pageCount = ref(0);
/** 页码 / 范围输入 */
const spec = ref("");

async function pickFile() {
  const p = await openDialog({ filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!p) return;
  pdfFile.value = String(p);
  spec.value = "";
  pageCount.value = await getPdfPageCount(String(p)).catch(() => 0);
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
async function handleDrop(paths: string[]) {
  const pdf = paths.find((p) => /\.pdf$/i.test(p));
  if (!pdf) {
    message.warning(t("organize.warnOnlyPdf"));
    return;
  }
  pdfFile.value = pdf;
  spec.value = "";
  pageCount.value = await getPdfPageCount(pdf).catch(() => 0);
}
defineExpose({ handleDrop });

/** 解析逗号分隔的页码/范围（支持中文逗号），返回页码数组；非法输入返回 null */
function parsePages(specStr: string): number[] | null {
  const pages: number[] = [];
  for (const part of specStr.split(/[,，]/).map((s) => s.trim()).filter(Boolean)) {
    const range = part.match(/^(\d+)\s*-\s*(\d+)$/);
    if (range) {
      const [a, b] = [Number(range[1]), Number(range[2])];
      if (a < 1 || a > b) return null;
      for (let i = a; i <= b; i++) pages.push(i);
    } else if (/^\d+$/.test(part)) {
      if (Number(part) < 1) return null;
      pages.push(Number(part));
    } else {
      return null;
    }
  }
  return pages.length ? pages : null;
}

/** 解析逗号分隔的删除范围（"2-4,7" → [[2,4],[7,7]]）；非法输入返回 null */
function parseRanges(specStr: string): [number, number][] | null {
  const ranges: [number, number][] = [];
  for (const part of specStr.split(/[,，]/).map((s) => s.trim()).filter(Boolean)) {
    const range = part.match(/^(\d+)\s*-\s*(\d+)$/);
    if (range) {
      const [a, b] = [Number(range[1]), Number(range[2])];
      if (a < 1 || a > b) return null;
      ranges.push([a, b]);
    } else if (/^\d+$/.test(part)) {
      if (Number(part) < 1) return null;
      ranges.push([Number(part), Number(part)]);
    } else {
      return null;
    }
  }
  return ranges.length ? ranges : null;
}

async function doWork() {
  if (!pdfFile.value) {
    message.warning(t("organize.warnNoFile"));
    return;
  }
  const specStr = spec.value.trim();
  const suffix = mode.value === "extract" ? "_extracted" : "_trimmed";
  const defaultName = pdfFile.value.replace(/\.pdf$/i, `${suffix}.pdf`).split(/[\\/]/).pop();
  const outPath = await openDialog({
    save: true,
    title: t(mode.value === "extract" ? "organize.saveTitleExtract" : "organize.saveTitleDelete"),
    defaultPath: defaultName,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!outPath) return;
  const kind = mode.value;
  try {
    const out =
      mode.value === "extract"
        ? await runExtract(specStr, String(outPath))
        : await runDelete(specStr, String(outPath));
    if (!out) return;
    const outName = out.split(/[\\/]/).pop() ?? out;
    message.success(t("organize.success", { name: outName }), { duration: 4000 });
    await history.add({ kind, name: outName, inputs: [pdfFile.value], outputs: [out], ok: true });
  } catch (e) {
    message.error(t("organize.fail", { err: String(e) }));
    await history.add({ kind, name: fileName.value, inputs: [pdfFile.value], outputs: [], ok: false });
  }
}

/** 提取页面：解析后调用后端；返回输出路径或 null（输入无效提前返回） */
async function runExtract(specStr: string, outPath: string): Promise<string | null> {
  const pages = parsePages(specStr);
  if (!pages) {
    message.warning(t("organize.warnInvalid"));
    return null;
  }
  if (pageCount.value > 0 && pages.some((p) => p > pageCount.value)) {
    message.warning(t("organize.warnOutOfRange", { n: pageCount.value }));
    return null;
  }
  return pdfExtractPages(pdfFile.value, outPath, pages);
}

/** 删除页面：解析后调用后端；返回输出路径或 null（输入无效提前返回） */
async function runDelete(specStr: string, outPath: string): Promise<string | null> {
  const ranges = parseRanges(specStr);
  if (!ranges) {
    message.warning(t("organize.warnInvalid"));
    return null;
  }
  if (pageCount.value > 0 && ranges.some(([a, b]) => b > pageCount.value)) {
    message.warning(t("organize.warnOutOfRange", { n: pageCount.value }));
    return null;
  }
  return pdfDeletePages(pdfFile.value, outPath, ranges);
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
.size-tag,
.pages-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 8px;
  color: var(--text-muted);
  background: var(--bg-tag);
  flex-shrink: 0;
}
.pages-tag {
  color: var(--accent);
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
.config-hint {
  font-size: 12px;
  color: var(--text-faint);
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
.cta:hover:not(:disabled) {
  opacity: 0.85;
}
.cta:disabled {
  background: var(--cta-disabled);
  cursor: not-allowed;
}
</style>
