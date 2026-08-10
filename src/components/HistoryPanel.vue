<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("history.title") }}</h2>
      <p>{{ t("history.subtitle") }}</p>
    </div>

    <!-- 工具栏 -->
    <div v-if="history.items.length" class="toolbar">
      <span class="count">{{ t("history.count", { n: history.items.length }) }}</span>
      <button class="clear-btn" @click="clearAll">
        <NIcon :component="TrashOutline" :size="15" />
        {{ t("history.clear") }}
      </button>
    </div>

    <!-- 空状态 -->
    <div v-if="!history.items.length" class="empty">
      <NIcon :component="TimeOutline" :size="34" color="var(--text-faint)" />
      <p class="empty-main">{{ t("history.empty") }}</p>
      <p class="empty-sub">{{ t("history.emptySub") }}</p>
    </div>

    <!-- 历史列表 -->
    <div v-else class="history-list">
      <div v-for="item in history.items" :key="item.id" class="history-row">
        <NIcon :component="kindIcon(item.kind)" :size="18" :color="item.ok ? kindColor(item.kind) : 'var(--red)'" />
        <div class="h-info">
          <div class="h-name">
            {{ item.name }}
            <span v-if="!item.ok" class="h-fail">{{ t("common.fail") }}</span>
          </div>
          <div class="h-meta">
            {{ t(`history.kind${kindLabel(item.kind)}`) }} · {{ formatTime(item.time) }}
          </div>
        </div>
        <template v-if="item.outputs.length">
          <button class="link-btn" @click="openPath(item.outputs[0])">{{ t("common.open") }}</button>
          <button class="link-btn" @click="openPath(dirOf(item.outputs[0]))">{{ t("common.openDir") }}</button>
        </template>
        <button class="remove-btn" :title="t('history.remove')" @click="history.remove(item.id)">×</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NIcon, useDialog, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import {
  ArchiveOutline, DocumentOutline, GitBranchOutline, GitMergeOutline, ImagesOutline, ImageOutline, GitCompareOutline, GlobeOutline, TextOutline, SparklesOutline,
  ListOutline, LockClosedOutline, LockOpenOutline, RefreshOutline,
  SwapHorizontalOutline, TimeOutline, TrashOutline, WaterOutline,
} from "@vicons/ionicons5";
import { openPath } from "../api";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const dialog = useDialog();
const history = useHistoryStore();

/** 各操作类型的图标与主题色 */
const KIND_META: Record<string, { icon: any; color: string; label: string }> = {
  merge: { icon: GitMergeOutline, color: "#e6494c", label: "Merge" },
  split: { icon: GitBranchOutline, color: "#e6494c", label: "Split" },
  compress: { icon: ArchiveOutline, color: "#e6494c", label: "Compress" },
  extract: { icon: DocumentOutline, color: "#e6494c", label: "Extract" },
  delete: { icon: TrashOutline, color: "#d03050", label: "Delete" },
  watermark: { icon: WaterOutline, color: "#2080f0", label: "Watermark" },
  rotate: { icon: RefreshOutline, color: "#722ed1", label: "Rotate" },
  pages: { icon: ListOutline, color: "#722ed1", label: "Pages" },
  encrypt: { icon: LockClosedOutline, color: "#d03050", label: "Encrypt" },
  decrypt: { icon: LockOpenOutline, color: "#18a058", label: "Decrypt" },
  images2pdf: { icon: ImagesOutline, color: "#e6494c", label: "Images" },
  convert: { icon: SwapHorizontalOutline, color: "#722ed1", label: "Convert" },
  watcher: { icon: TimeOutline, color: "#18a058", label: "Watcher" },
  pdfExtractImages: { icon: ImageOutline, color: "#722ed1", label: "PdfExtractImages" },
  removeWatermark: { icon: WaterOutline, color: "#2080f0", label: "RemoveWatermark" },
  comparePdf: { icon: GitCompareOutline, color: "#722ed1", label: "ComparePdf" },
  webToPdf: { icon: GlobeOutline, color: "#2080f0", label: "WebToPdf" },
  batchRename: { icon: TextOutline, color: "#722ed1", label: "BatchRename" },
  aiSummary: { icon: SparklesOutline, color: "#722ed1", label: "AiSummary" },
};

function kindIcon(kind: string): any {
  return KIND_META[kind]?.icon ?? SwapHorizontalOutline;
}
function kindColor(kind: string): string {
  return KIND_META[kind]?.color ?? "#2080f0";
}
/** 生成 i18n key 后缀（kind.xxx → history.kindXxx） */
function kindLabel(kind: string): string {
  return KIND_META[kind]?.label ?? "Convert";
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function dirOf(p: string): string {
  const i = p.lastIndexOf("/");
  return i >= 0 ? p.slice(0, i) : p;
}

function clearAll() {
  dialog.warning({
    title: t("history.clear"),
    content: t("history.clearConfirm"),
    positiveText: t("history.clearOk"),
    negativeText: t("history.clearCancel"),
    onPositiveClick: () => {
      void history.clear();
      message.success(t("history.cleared"));
    },
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
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 16px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-soft);
}
.count {
  font-size: 12px;
  color: var(--text-muted);
}
.clear-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--border-strong);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 12px;
  padding: 5px 12px;
  border-radius: 7px;
  cursor: pointer;
  transition: all 0.15s;
}
.clear-btn:hover {
  border-color: var(--red);
  color: var(--red);
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-top: 18px;
  padding: 44px 28px;
  border: 2px dashed var(--border-dash);
  border-radius: 14px;
}
.empty-main {
  margin: 0;
  font-size: 14px;
  color: var(--text-sub);
}
.empty-sub {
  margin: 0;
  font-size: 12px;
  color: var(--text-faint);
}
.history-list {
  margin-top: 6px;
}
.history-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 6px;
  border-top: 1px solid var(--border-soft);
}
.h-info {
  flex: 1;
  min-width: 0;
}
.h-name {
  font-size: 13px;
  color: var(--text-body);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.h-fail {
  margin-left: 6px;
  font-size: 11px;
  color: var(--red);
  border: 1px solid var(--red);
  border-radius: 6px;
  padding: 0 5px;
}
.h-meta {
  margin-top: 2px;
  font-size: 11px;
  color: var(--text-faint);
}
.link-btn {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
  white-space: nowrap;
}
.link-btn:hover { text-decoration: underline; }
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
</style>
