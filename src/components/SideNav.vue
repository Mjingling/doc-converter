<template>
  <aside class="side-nav">
    <!-- 功能导航 -->
    <nav class="nav">
      <!-- 顶部搜索：按名称过滤全部导航项 -->
      <div class="search-box">
        <NIcon :component="SearchOutline" :size="14" />
        <input
          v-model="searchQuery"
          class="search-input"
          :placeholder="t('nav.searchPlaceholder')"
          spellcheck="false"
          @keydown="onSearchKeydown"
        />
        <button v-if="searchQuery" class="search-clear" @click="searchQuery = ''">
          <NIcon :component="CloseOutline" :size="12" />
        </button>
      </div>
      <div v-for="g in filteredGroups" :key="g.title" class="nav-group">
        <div v-if="g.title !== 'nav.groupBottom'" class="group-title">
          <span>{{ t(g.title) }}</span>
          <span v-if="g.engine !== 'none'" class="engine-tag" :class="g.engine">
            {{ g.engine === "builtin" ? t("common.builtin") : t("common.libreoffice") }}
          </span>
        </div>
        <div
          v-for="item in g.items"
          :key="item.id"
          class="nav-item"
          :class="{ active: item.id === active, 'kb-active': item.id === kbActiveId }"
          @click="activate(item.id)"
        >
          <NIcon :component="item.icon" :size="17" :color="item.color" />
          <span class="nav-label">{{ t(item.label) }}</span>
          <span v-if="g.engine === 'libreoffice' && !engine.available" class="need-engine">{{ t("common.needInstall") }}</span>
        </div>
      </div>
    </nav>

    <!-- 引擎切换 + 捐赠：轻量单行入口 -->
    <div class="engine-card">
      <div class="engine-row">
        <span class="engine-dot" :class="engine.mode === 'libreoffice' ? 'on' : ''"></span>
        <span class="engine-name" :title="engineDesc">
          {{ engine.mode === "libreoffice" ? t("engine.nameLo") : t("engine.nameBuiltin") }}
        </span>
        <button class="switch-link" :title="switchBtnLabel" @click="toggleEngine" :disabled="switching">
          <NIcon :component="SwapHorizontalOutline" :size="13" />
          {{ t("common.switch") }}
        </button>
        <span class="bar-sep"></span>
        <button class="donate-link" @click="showDonate = true">
          <NIcon :component="HeartOutline" :size="13" /> {{ t("common.donate") }}
        </button>
        <span class="bar-sep"></span>
        <button class="check-link" @click="showUpdate = true" :title="t('update.checkBtn')">
          <NIcon :component="RefreshOutline" :size="13" />
        </button>
      </div>
      <!-- 内置引擎未安装：警告 + 补救链接 -->
      <div v-if="engine.mode === 'builtin' && !engine.available" class="card-links">
        <span class="warn-text">{{ t("engine.descMissing") }}</span>
        <button class="link-btn" @click="redetect">{{ t("common.redetect") }}</button>
        <button class="link-btn" @click="openDownload">{{ t("common.download") }}</button>
      </div>
    </div>

    <!-- 捐赠 + 检查更新 -->

    <DonateModal v-model:show="showDonate" />
    <UpdateModal v-model:show="showUpdate" />
  </aside>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick } from "vue";
import { NIcon, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  GitMergeOutline, GitBranchOutline, ArchiveOutline,
  DocumentTextOutline, ImageOutline, DocumentOutline,
  GridOutline, EaselOutline, SwapHorizontalOutline,
  HeartOutline, WaterOutline, RefreshOutline, LockClosedOutline,
  ImagesOutline, TimeOutline, CopyOutline, CutOutline,
  InformationCircleOutline, ResizeOutline, BookmarkOutline,
  DocumentAttachOutline, ContractOutline,
  GlobeOutline, TextOutline, SparklesOutline,
  SearchOutline, CloseOutline, SettingsOutline,
} from "@vicons/ionicons5";
import { useEngineStore } from "../stores/engine";
import DonateModal from "./DonateModal.vue";
import UpdateModal from "./UpdateModal.vue";
import type { NavId } from "../types";

const { t } = useI18n();
const message = useMessage();
const engine = useEngineStore();

/** 捐赠弹窗开关 */
const showDonate = ref(false);
/** 检查更新弹窗开关 */
const showUpdate = ref(false);
/** 切换/检测引擎时的忙碌状态 */
const switching = ref(false);

/** 引擎卡片描述文案 */
const engineDesc = computed(() => {
  if (engine.mode === "libreoffice") return t("engine.descLo");
  return engine.available ? t("engine.descBuiltin") : t("engine.descMissing");
});

/** 引擎切换按钮文案 */
const switchBtnLabel = computed(() => {
  if (engine.mode === "libreoffice") return t("engine.switchBack");
  return engine.available ? t("engine.switchTo") : t("engine.redetectSwitch");
});

const props = defineProps<{
  active: NavId;
}>();

const emit = defineEmits<{
  (e: "select", id: NavId): void;
}>();

/** 导航分组：PDF 处理 + PDF 工具箱（内置引擎）+ 文档转换（LibreOffice）+ 历史记录；label 为 i18n key */
const groups: {
  title: string;
  engine: "builtin" | "libreoffice" | "none";
  items: { id: NavId; label: string; icon: any; color: string }[];
}[] = [
  // 历史记录放最前：高频入口，避免被 21 个功能项挤到列表底部
  {
    title: "nav.groupHistory",
    engine: "none",
    items: [{ id: "history", label: "nav.history", icon: TimeOutline, color: "#e6494c" }],
  },
  {
    title: "nav.groupPdf",
    engine: "builtin",
    items: [
      { id: "merge", label: "nav.merge", icon: GitMergeOutline, color: "#e6494c" },
      { id: "split", label: "nav.split", icon: GitBranchOutline, color: "#e6494c" },
      { id: "compress", label: "nav.compress", icon: ArchiveOutline, color: "#e6494c" },
      { id: "organize", label: "nav.organize", icon: CutOutline, color: "#e6494c" },
    ],
  },
  {
    title: "nav.groupTools",
    engine: "builtin",
    items: [
      { id: "watermark", label: "nav.watermark", icon: WaterOutline, color: "#e6494c" },
      { id: "rotate", label: "nav.rotate", icon: RefreshOutline, color: "#e6494c" },
      { id: "encrypt", label: "nav.encrypt", icon: LockClosedOutline, color: "#e6494c" },
      { id: "images2pdf", label: "nav.images2pdf", icon: ImagesOutline, color: "#e6494c" },
      { id: "batch", label: "nav.batch", icon: CopyOutline, color: "#e6494c" },
    ],
  },
  {
    title: "nav.groupConvert",
    engine: "libreoffice",
    items: [
      { id: "pdf2word", label: "nav.pdf2word", icon: DocumentTextOutline, color: "#2080f0" },
      { id: "pdf2image", label: "nav.pdf2image", icon: ImageOutline, color: "#2080f0" },
      { id: "pdf2excel", label: "nav.pdf2excel", icon: GridOutline, color: "#2080f0" },
      { id: "word2pdf", label: "nav.word2pdf", icon: DocumentOutline, color: "#2080f0" },
      { id: "excel2pdf", label: "nav.excel2pdf", icon: GridOutline, color: "#2080f0" },
      { id: "ppt2pdf", label: "nav.ppt2pdf", icon: EaselOutline, color: "#2080f0" },
      { id: "convert", label: "nav.convert", icon: SwapHorizontalOutline, color: "#2080f0" },
    ],
  },
  {
    title: "nav.groupExtras",
    engine: "builtin",
    items: [
      { id: "metadata", label: "nav.metadata", icon: InformationCircleOutline, color: "#e6494c" },
      { id: "crop", label: "nav.crop", icon: ResizeOutline, color: "#e6494c" },
      { id: "outline", label: "nav.outline", icon: BookmarkOutline, color: "#e6494c" },
      { id: "pdfExtractImages", label: "nav.pdfExtractImages", icon: ImageOutline, color: "#e6494c" },
      { id: "removeWatermark", label: "nav.removeWatermark", icon: WaterOutline, color: "#e6494c" },
      { id: "comparePdf", label: "nav.comparePdf", icon: DocumentTextOutline, color: "#e6494c" },
    ],
  },
  {
    title: "nav.groupUtils",
    engine: "none",
    items: [
      { id: "webToPdf", label: "nav.webToPdf", icon: GlobeOutline, color: "#18a058" },
      { id: "docxExtract", label: "nav.docxExtract", icon: DocumentAttachOutline, color: "#18a058" },
      { id: "imageCompress", label: "nav.imageCompress", icon: ContractOutline, color: "#18a058" },
      { id: "batchRename", label: "nav.batchRename", icon: TextOutline, color: "#18a058" },
      { id: "aiSummary", label: "nav.aiSummary", icon: SparklesOutline, color: "#18a058" },
    ],
  },
  // 设置（底部）
  {
    title: "nav.groupBottom",
    engine: "none",
    items: [
      { id: "settings", label: "nav.settings", icon: SettingsOutline, color: "var(--text-muted)" },
    ],
  },
];

/** 顶部搜索关键词：按 i18n 名称 / id 过滤导航项 */
const searchQuery = ref("");
const filteredGroups = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return groups;
  return groups
    .map((g) => ({
      ...g,
      items: g.items.filter(
        (i) => t(i.label).toLowerCase().includes(q) || i.id.toLowerCase().includes(q)
      ),
    }))
    .filter((g) => g.items.length > 0);
});

/* ---------- 键盘上下选择搜索结果 ---------- */

/** 搜索结果拍平后的 id 列表（供键盘上下遍历） */
const flatResults = computed(() => filteredGroups.value.flatMap((g) => g.items.map((i) => i.id)));

/** 键盘当前索引（-1 = 未激活键盘导航） */
const kbIndex = ref(-1);

/** 键盘高亮项 id */
const kbActiveId = computed(() =>
  kbIndex.value >= 0 ? (flatResults.value[kbIndex.value] ?? null) : null
);

/** 选中搜索结果：触发切换 + 清空搜索并退出键盘导航 */
function activate(id: NavId) {
  emit("select", id);
  searchQuery.value = "";
  kbIndex.value = -1;
}

/** 搜索框键盘事件：↑/↓ 遍历结果，Enter 选中，Esc 清空 */
function onSearchKeydown(e: KeyboardEvent) {
  // 中文输入法组合期间（选候选词等）不拦截方向键/回车
  if (e.isComposing) return;
  const n = flatResults.value.length;
  if (e.key === "ArrowDown") {
    if (n === 0) return;
    e.preventDefault();
    kbIndex.value = kbIndex.value < 0 ? 0 : Math.min(kbIndex.value + 1, n - 1);
  } else if (e.key === "ArrowUp") {
    if (n === 0) return;
    e.preventDefault();
    kbIndex.value = kbIndex.value < 0 ? n - 1 : Math.max(kbIndex.value - 1, 0);
  } else if (e.key === "Enter") {
    const target = kbIndex.value >= 0 ? kbIndex.value : 0;
    const id = flatResults.value[target];
    if (id) {
      e.preventDefault();
      activate(id);
    }
  } else if (e.key === "Escape") {
    searchQuery.value = "";
  }
}

/** 搜索词变化时退出键盘导航 */
watch(searchQuery, () => {
  kbIndex.value = -1;
});

/** 键盘高亮项变化时滚动到可见区域 */
watch(kbActiveId, async (id) => {
  if (!id) return;
  await nextTick();
  document
    .querySelector(".nav-item.kb-active")
    ?.scrollIntoView({ block: "nearest" });
});

/** 切换引擎模式；切换前先重新检测，用户可能刚安装完 LibreOffice */
async function toggleEngine() {
  if (switching.value) return;
  if (engine.mode === "builtin") {
    switching.value = true;
    try {
      await engine.refresh();
    } catch {
      /* 检测失败时按未安装处理 */
    }
    switching.value = false;
    if (engine.useLibreOffice()) {
      message.success(t("engine.msgSwitchedLo"));
    } else {
      message.warning(t("engine.msgMissingLo"), { duration: 5000 });
    }
  } else {
    engine.useBuiltin();
    message.info(t("engine.msgBackBuiltin"));
  }
}

/** 仅重新检测 LibreOffice 是否已安装 */
async function redetect() {
  switching.value = true;
  try {
    const ok = await engine.refresh();
    message[ok ? "success" : "warning"](
      ok ? t("engine.msgDetected", { action: t("engine.switchTo") }) : t("engine.msgNotDetected"),
      { duration: 4000 }
    );
  } catch {
    message.error(t("engine.msgDetectFailed"));
  } finally {
    switching.value = false;
  }
}

/** 打开 LibreOffice 下载页 */
function openDownload() {
  void openUrl("https://www.libreoffice.org/download/");
}
</script>

<style scoped>
.side-nav {
  width: 236px;
  flex-shrink: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
  padding: 14px;
  box-sizing: border-box;
}
/* 导航 */
.nav {
  flex: 1;
  overflow-y: auto;
}
.nav-group {
  margin-bottom: 14px;
}
.group-title {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  margin-bottom: 4px;
  font-size: 12px;
  color: var(--text-muted);
}
.engine-tag {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  line-height: 16px;
}
.engine-tag.builtin {
  color: var(--green);
  background: var(--green-soft);
}
.engine-tag.libreoffice {
  color: var(--accent);
  background: var(--accent-soft);
}
/* 顶部搜索框 */
.search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  padding: 0 8px;
  margin-bottom: 12px;
  background: var(--bg-input);
  color: var(--text-faint);
}
.search-box:focus-within {
  border-color: var(--accent);
}
.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-size: 13px;
  color: var(--text-main);
  padding: 6px 0;
}
.search-input::placeholder {
  color: var(--text-faint);
}
.search-clear {
  display: inline-flex;
  align-items: center;
  border: none;
  background: none;
  color: var(--text-faint);
  cursor: pointer;
  padding: 2px;
}
.search-clear:hover {
  color: var(--text-main);
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-radius: 8px;
  cursor: pointer;
  color: var(--text-sub);
  font-size: 14px;
  transition: background 0.15s;
  margin-bottom: 2px;
}
.nav-item:hover {
  background: var(--bg-hover);
}
.nav-item.active {
  background: var(--bg-active);
  font-weight: 600;
  color: var(--text-main);
}
/* 键盘上下选中的搜索结果项 */
.nav-item.kb-active {
  background: var(--bg-hover);
  color: var(--text-main);
  box-shadow: inset 0 0 0 1.5px var(--accent);
}
.nav-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.need-engine {
  font-size: 10px;
  color: var(--orange);
  background: var(--orange-soft);
  padding: 1px 6px;
  border-radius: 8px;
  line-height: 16px;
}
/* 引擎卡片：轻量单行（状态 + 切换 + 捐赠） */
.engine-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 8px 10px;
  margin-top: 8px;
  background: var(--bg-input);
}
.engine-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.engine-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--green);
  flex-shrink: 0;
}
.engine-dot.on {
  background: var(--accent);
  box-shadow: 0 0 0 3px rgba(32, 128, 240, 0.15);
}
.engine-name {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-main);
}
.switch-link {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
  border: 1px solid var(--border-strong);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}
.switch-link:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.switch-link:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.bar-sep {
  flex-shrink: 0;
  width: 1px;
  height: 12px;
  background: var(--border-strong);
}
.donate-link,
.check-link {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
  border: none;
  background: none;
  padding: 0;
  font-size: 12px;
  color: var(--text-muted);
  cursor: pointer;
}
.donate-link:hover,
.check-link:hover {
  color: var(--accent);
}
/* 内置引擎未安装：警告 + 补救链接 */
.card-links {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--border-soft);
}
.warn-text {
  font-size: 11px;
  color: var(--orange);
}
.link-btn {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
}
.link-btn:hover {
  text-decoration: underline;
}
</style>
