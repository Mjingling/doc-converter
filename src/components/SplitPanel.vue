<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("split.title") }}</h2>
      <p>{{ t("split.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile">
      <div v-if="!splitFile" class="zone-empty">
        <div class="zone-icons">
          <NIcon :component="GitBranchOutline" :size="34" color="#e6494c" />
        </div>
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("split.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="splitFile">{{ splitFileName }}</span>
        <span class="page-tag" :class="splitPageCount ? 'ok' : 'fail'">
          {{ splitPageCount ? t("split.pages", { n: splitPageCount }) : t("split.pagesFail") }}
        </span>
      </div>
    </div>

    <!-- 页范围配置 -->
    <div v-if="splitFile" class="config">
      <div class="config-label">
        <span class="range-label">
          {{ t("split.rangeLabel") }}
          <span class="config-count">{{ t("split.previewTitle", { n: previewCount }) }}</span>
        </span>
        <span class="range-tools">
          <button class="add-range" @click="ranges.push({ start: 1, end: 1 })">{{ t("split.addRange") }}</button>
          <span class="tool-label">{{ t("split.perSegment") }}</span>
          <NInputNumber
            v-model:value="pagesPerRange"
            :min="1"
            :max="splitPageCount || 99999"
            size="small"
            style="width: 76px"
          />
          <span class="tool-label">{{ t("split.pagesUnit") }}</span>
          <button class="add-range auto" :disabled="!splitPageCount" @click="autoGenRanges">
            {{ t("split.autoGen") }}
          </button>
        </span>
      </div>
      <div v-for="(r, i) in ranges" :key="i" class="range-row">
        <NInputNumber
          v-model:value="r.start"
          :min="1"
          :max="splitPageCount || 99999"
          size="small"
          :placeholder="t('split.startPlaceholder')"
          style="width: 110px"
        />
        <span class="dash">—</span>
        <NInputNumber
          v-model:value="r.end"
          :min="1"
          :max="splitPageCount || 99999"
          size="small"
          :placeholder="t('split.endPlaceholder')"
          style="width: 110px"
        />
        <span class="range-preview" :class="{ invalid: !previewNames[i]?.valid }" :title="previewNames[i]?.name || t('split.previewInvalid')">
          <NIcon v-if="previewNames[i]?.valid" :component="DocumentTextOutline" :size="13" color="#18a058" />
          <NIcon v-else :component="WarningOutline" :size="13" color="#d03050" />
          {{ previewNames[i]?.valid ? midEllipsis(previewNames[i]!.name) : t("split.previewInvalid") }}
        </span>
        <button class="remove-btn" :disabled="ranges.length <= 1" @click="ranges.splice(i, 1)">×</button>
      </div>

    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("split.naming") }}</span>
      <button class="cta" :disabled="!splitFile || running" @click="doSplit">
        <NIcon :component="GitBranchOutline" :size="17" />
        {{ running ? t("split.running") : t("split.cta") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="running" indeterminate :label="t('split.running')" />

    <!-- 拆分结果 -->
    <div v-if="splitResults.length" class="task-list">
      <div class="task-title">{{ t("split.resultTitle", { n: splitResults.length }) }}</div>
      <div v-for="r in splitResults" :key="r" class="task-row">
        <NIcon :component="DocumentTextOutline" :size="17" color="#18a058" />
        <span class="fname">{{ r.split(/[\\/]/).pop() }}</span>
        <button class="link-btn" @click="openPath(r)">{{ t("common.open") }}</button>
        <button class="link-btn" @click="openPath(dirOf(r))">{{ t("common.openDir") }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInputNumber, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { DocumentTextOutline, GitBranchOutline, WarningOutline } from "@vicons/ionicons5";
import { getPdfPageCount, openPath, pdfSplit } from "../api";
import { dirOf, defaultOutDir } from "../utils/file";
import { usePanelTask } from "../composables/usePanelTask";
import TaskProgress from "./TaskProgress.vue";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const history = useHistoryStore();

const splitFile = ref("");
const splitPageCount = ref(0);
const ranges = ref<{ start: number | null; end: number | null }[]>([{ start: 1, end: 1 }]);
/** 自动生成范围时每段的固定页数 */
const pagesPerRange = ref(5);
const splitResults = ref<string[]>([]);
const splitFileName = computed(() => splitFile.value.split(/[\\/]/).pop() ?? splitFile.value);

/** 执行状态：running + 进度条 */
const { running, run } = usePanelTask({ panelId: "split", label: t("split.title") });

/** 输出文件预览：原文件名_页码范围.pdf（单页省略连字符，如 report_5.pdf）；范围无效时标记 */
const previewNames = computed(() => {
  if (!splitFile.value) return [];
  const stem = (splitFile.value.split(/[\\/]/).pop() || "").replace(/\.pdf$/i, "") || "split";
  return ranges.value.map((r) => {
    if (r.start == null || r.end == null || r.start < 1 || r.start > r.end) {
      return { valid: false, name: "" };
    }
    const range = r.start === r.end ? String(r.start) : `${r.start}-${r.end}`;
    return { valid: true, name: `${stem}_${range}.pdf` };
  });
});

/** 有效范围数量（无效范围不计入） */
const previewCount = computed(() => previewNames.value.filter((p) => p.valid).length);

/** 文件名中间省略：超长时保留头部（原文件名）与尾部（页码范围），hover 可见全名 */
function midEllipsis(name: string, max = 36): string {
  if (name.length <= max) return name;
  const head = Math.ceil(max * 0.6);
  const tail = max - head - 1;
  return `${name.slice(0, head)}…${name.slice(-tail)}`;
}

function resetSplit() {
  splitFile.value = "";
  splitPageCount.value = 0;
  splitResults.value = [];
  ranges.value = [{ start: 1, end: 1 }];
}

async function loadPageCount() {
  splitPageCount.value = 0;
  try {
    splitPageCount.value = await getPdfPageCount(splitFile.value);
  } catch (e) {
    message.warning(t("split.pageCountFail", { err: String(e) }));
  }
}

async function pickFile() {
  const p = await openDialog({
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!p) return;
  splitFile.value = String(p);
  splitResults.value = [];
  ranges.value = [{ start: 1, end: 1 }];
  await loadPageCount();
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
function handleDrop(paths: string[]) {
  const pdf = paths.find((p) => /\.pdf$/i.test(p));
  if (!pdf) {
    message.warning(t("split.warnOnlyPdf"));
    return;
  }
  resetSplit();
  splitFile.value = pdf;
  void loadPageCount();
}
defineExpose({ handleDrop });

/** 按每段固定页数自动生成页范围（最后一段不足时按剩余页数） */
function autoGenRanges() {
  const total = splitPageCount.value;
  const n = pagesPerRange.value || 1;
  if (total <= 0) {
    message.warning(t("split.warnNoPageCount"));
    return;
  }
  const rs: { start: number | null; end: number | null }[] = [];
  for (let s = 1; s <= total; s += n) {
    rs.push({ start: s, end: Math.min(s + n - 1, total) });
  }
  ranges.value = rs;
}

async function doSplit() {
  if (!splitFile.value) {
    message.warning(t("split.warnNoFile"));
    return;
  }
  const dir = defaultOutDir(splitFile.value, settings.defaultOutDir);
  const rs: [number, number][] = [];
  for (const r of ranges.value) {
    if (r.start == null || r.end == null) {
      message.warning(t("split.warnRangeEmpty"));
      return;
    }
    if (r.start > r.end) {
      message.warning(t("split.warnRangeInvalid", { s: r.start, e: r.end }));
      return;
    }
    rs.push([r.start, r.end]);
  }
  // 检测页范围重叠
  const sorted = [...rs].sort((a, b) => a[0] - b[0]);
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i][0] <= sorted[i - 1][1]) {
      message.warning(t("split.warnOverlap", { s: sorted[i][0], e: sorted[i][1] }));
      return;
    }
  }
  await run(async () => {
    try {
      const outs = await pdfSplit(splitFile.value, rs, dir);
      splitResults.value = outs;
      message.success(t("split.success", { n: outs.length }), { duration: 4000 });
      await history.add({
        kind: "split",
        name: `${splitFileName.value} → ${outs.length} ${t("split.pagesUnit")}`,
        inputs: [splitFile.value],
        outputs: outs,
        ok: true,
      });
    } catch (e) {
      message.error(t("split.fail", { err: String(e) }));
      await history.add({ kind: "split", name: splitFileName.value, inputs: [splitFile.value], outputs: [], ok: false });
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
.page-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 8px;
  flex-shrink: 0;
}
.page-tag.ok {
  color: var(--green);
  background: var(--green-soft);
}
.page-tag.fail {
  color: var(--red);
  background: var(--red-soft);
}
.config {
  margin-top: 16px;
}
.config-label {
  font-size: 13px;
  color: var(--text-sub);
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.config-count {
  font-size: 12px;
  color: var(--text-muted);
  flex-shrink: 0;
}
.range-label {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
}
.range-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.range-preview {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--green);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.range-preview.invalid {
  color: var(--red);
}
.dash {
  color: var(--text-muted);
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
.remove-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
.remove-btn:hover:not(:disabled) { color: var(--red); }
.add-range {
  border: 1px dashed var(--text-faint);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 13px;
  padding: 5px 14px;
  border-radius: 8px;
  cursor: pointer;
}
.add-range:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.range-tools {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.tool-label {
  font-size: 12px;
  color: var(--text-muted);
}
.add-range.auto {
  border-style: solid;
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}
.add-range.auto:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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
.task-list {
  margin-top: 18px;
}
.task-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-sub);
  margin-bottom: 4px;
}
.task-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 6px;
  border-top: 1px solid var(--border-soft);
  font-size: 13px;
}
.task-row .fname {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--text-body);
}
.link-btn {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
}
.link-btn:hover { text-decoration: underline; }
</style>
