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
        <div v-if="g.title && g.title !== 'nav.groupBottom'" class="group-title">
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

    <!-- 任务池指示器：后台任务运行中/刚完成时显示，点击展开任务列表 -->
    <div
      v-if="poolVisible"
      class="task-pool"
      :class="{ 'all-done': runningCount === 0 }"
      @click="poolOpen = !poolOpen"
    >
      <span v-if="runningCount > 0" class="pool-spin"></span>
      <NIcon v-else :component="CheckmarkCircleOutline" :size="13" class="pool-done" />
      <span class="pool-text">
        {{ runningCount > 0
          ? t("taskPool.running", { n: runningCount })
          : t("taskPool.done", { n: finishedCount }) }}
      </span>
      <!-- 任务列表浮层：名称 + 进度 + 前往对应面板 -->
      <Transition name="pool">
        <div v-if="poolOpen" class="pool-popover" @click.stop>
          <div v-for="task in pool.tasks" :key="task.id" class="pool-task">
            <span class="pool-dot" :class="task.running ? 'run' : task.ok ? 'ok' : 'fail'"></span>
            <span class="pool-task-name" :title="task.label">{{ task.label }}</span>
            <span class="pool-task-progress">
              {{ task.running ? (task.progress != null ? task.progress + "%" : "…") : task.ok ? "✓" : "×" }}
            </span>
            <button class="pool-goto" @click="goToPanel(task.panelId)">{{ t("taskPool.goTo") }}</button>
          </div>
        </div>
      </Transition>
    </div>

    <!-- 引擎切换 + 捐赠：轻量单行入口 -->
    <div class="engine-card">
      <div class="engine-row">
        <span class="engine-dot" :class="engine.mode === 'libreoffice' ? 'on' : ''"></span>
        <span class="engine-name" :title="engineDesc">
          {{ engine.mode === "libreoffice" ? t("engine.nameLo") : t("engine.nameBuiltin") }}
        </span>
        <button class="switch-link" :title="switchBtnLabel" @click="toggleEngine" :disabled="switching">
          <NIcon :component="SwapHorizontalOutline" :size="13" />
        </button>
        <span class="bar-sep"></span>
        <button class="donate-link" :title="t('common.donate')" @click="showDonate = true">
          <NIcon :component="HeartOutline" :size="13" />
        </button>
        <span class="bar-sep"></span>
        <n-dropdown
          trigger="click"
          placement="top-start"
          :options="settingsMenuOptions"
          @select="onSettingsMenuSelect"
        >
          <button class="settings-link" :title="t('common.settings')">
            <NIcon :component="SettingsOutline" :size="13" />
          </button>
        </n-dropdown>
      </div>
      <!-- 内置引擎未安装：警告 + 补救链接 -->
      <div v-if="engine.mode === 'builtin' && !engine.available" class="card-links">
        <span class="warn-text">{{ t("engine.descMissing") }}</span>
        <button class="link-btn" @click="redetect">{{ t("common.redetect") }}</button>
        <button class="link-btn" @click="openDownload">{{ t("common.download") }}</button>
      </div>
    </div>

    <!-- 捐赠 + 设置菜单 -->

    <DonateModal v-model:show="showDonate" />
  </aside>
</template>

<script setup lang="ts">
import { computed, h, onBeforeUnmount, onMounted, ref, watch, nextTick } from "vue";
import { NDropdown, NIcon, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  HeartOutline, SwapHorizontalOutline, ColorPaletteOutline, HappyOutline, LanguageOutline,
  SearchOutline, CloseOutline, SettingsOutline, CheckmarkCircleOutline,
} from "@vicons/ionicons5";
import { useEngineStore } from "../stores/engine";
import { useSettingsStore } from "../stores/settings";
import { useTaskPoolStore } from "../stores/taskPool";
import type { AppLocale, AppTheme } from "../stores/settings";
import DonateModal from "./DonateModal.vue";
import { navGroups } from "../navItems";
import type { NavId } from "../types";

const { t } = useI18n();
const message = useMessage();
const engine = useEngineStore();

/* ---------- 任务池指示器：后台任务全局可见（数量 + 列表浮层 + 前往） ---------- */
const pool = useTaskPoolStore();
const poolOpen = ref(false);
const runningCount = computed(() => pool.runningTasks.length);
const finishedCount = computed(() => pool.justFinished.length);
const poolVisible = computed(() => runningCount.value > 0 || finishedCount.value > 0);

/** 浮层内「前往」：跳到任务所属面板并收起浮层 */
function goToPanel(id: NavId) {
  poolOpen.value = false;
  emit("select", id);
}

// 任务全部结束（完成提示驻留期过后）自动收起浮层与指示器
watch(runningCount, (n) => {
  if (n === 0) window.setTimeout(() => (poolOpen.value = false), 100);
});

let sweepTimer = 0;
onMounted(() => {
  // 定期清理驻留窗口外的已完成任务，防止列表无限增长
  sweepTimer = window.setInterval(() => pool.sweep(), 1000);
});
onBeforeUnmount(() => window.clearInterval(sweepTimer));

/** 捐赠弹窗开关 */
const showDonate = ref(false);
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

/* ---------- 左下角设置菜单：设置 / 界面语言 / 主题 ---------- */

const settings = useSettingsStore();

/** 语言选项（与设置面板语言下拉一致）：label 字面量或 labelKey i18n 二选一 */
interface LocaleOption {
  value: AppLocale;
  label?: string;
  labelKey?: string;
}
const LOCALE_OPTIONS: LocaleOption[] = [
  { value: "system", labelKey: "settings.followSystem" },
  { value: "zh-CN", label: "简体中文" },
  { value: "en-US", label: "English" },
  { value: "ja-JP", label: "日本語" },
  { value: "ko-KR", label: "한국어" },
];

/** 主题选项：浅色 / 深色 / 跟随系统 */
const THEME_OPTIONS = [
  { value: "light", labelKey: "settings.light" },
  { value: "dark", labelKey: "settings.dark" },
  { value: "system", labelKey: "settings.followSystem" },
] as const;

/** 菜单项图标（NDropdown 渲染函数形式） */
const menuIcon = (icon: any) => () => h(NIcon, { component: icon, size: 14 });

/** 当前值带 ✓ 前缀（直观展示选中态） */
const settingsMenuOptions = computed(() => [
  { label: t("common.settings"), key: "open-settings", icon: menuIcon(SettingsOutline) },
  {
    label: t("settings.language"),
    key: "language",
    icon: menuIcon(LanguageOutline),
    children: LOCALE_OPTIONS.map((l) => ({
      label: (settings.locale === l.value ? "✓ " : "") + (l.labelKey ? t(l.labelKey) : (l.label ?? "")),
      key: `locale:${l.value}`,
    })),
  },
  {
    label: t("settings.theme"),
    key: "theme",
    icon: menuIcon(ColorPaletteOutline),
    children: THEME_OPTIONS.map((th) => ({
      label: (settings.theme === th.value ? "✓ " : "") + t(th.labelKey),
      key: `theme:${th.value}`,
    })),
  },
  {
    label: settings.pet.enabled ? t("pet.menu.hide") : t("pet.menu.show"),
    key: "toggle-pet",
    icon: menuIcon(HappyOutline),
  },
]);

/** 设置菜单选择：打开设置面板 / 切换语言 / 切换主题 / 显示隐藏宠物 */
function onSettingsMenuSelect(key: string | number) {
  const k = String(key);
  if (k === "open-settings") {
    emit("select", "settings");
  } else if (k.startsWith("locale:")) {
    settings.setLocale(k.slice(7) as AppLocale);
  } else if (k.startsWith("theme:")) {
    settings.setTheme(k.slice(6) as AppTheme);
  } else if (k === "toggle-pet") {
    void togglePet();
  }
}

/** 切换桌面宠物显示/隐藏（持久化并即时创建/关闭宠物窗口） */
async function togglePet() {
  try {
    const on = !settings.pet.enabled;
    await settings.setPetEnabled(on);
    if (on) message.success(t("settings.msgPetOn"));
    else message.info(t("settings.msgPetOff"));
  } catch (e) {
    message.error(String(e));
  }
}

/** 导航分组数据抽取到 navItems.ts（与命令面板共用） */
const groups = navGroups;

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
  width: 264px;
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
/* 任务池指示器：运行中转圈计数 / 全部完成打勾，点击展开列表浮层 */
.task-pool {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 12px 8px;
  padding: 7px 12px;
  border: 1px solid var(--accent);
  background: var(--accent-soft);
  border-radius: 10px;
  font-size: 12px;
  color: var(--text-sub);
  cursor: pointer;
  user-select: none;
  flex-shrink: 0;
}
.task-pool.all-done {
  border-color: var(--green);
  background: var(--green-soft);
  color: var(--green);
}
.pool-spin {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 2px solid var(--accent);
  border-top-color: transparent;
  animation: pool-rotate 0.9s linear infinite;
  flex-shrink: 0;
}
.pool-done {
  color: var(--green);
  flex-shrink: 0;
}
.pool-text {
  flex: 1;
  min-width: 0;
}
/* 浮层：向上弹出 */
.pool-popover {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  right: 0;
  background: var(--bg-panel);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  box-shadow: 0 4px 14px var(--shadow);
  padding: 6px;
  z-index: 30;
  max-height: 260px;
  overflow-y: auto;
}
.pool-task {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  font-size: 12px;
}
.pool-task:hover {
  background: var(--bg-hover);
}
.pool-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}
.pool-dot.run {
  background: var(--accent);
  animation: pool-blink 1.1s ease-in-out infinite;
}
.pool-dot.ok {
  background: var(--green);
}
.pool-dot.fail {
  background: var(--red);
}
.pool-task-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-body);
}
.pool-task-progress {
  font-size: 11px;
  color: var(--text-muted);
  flex-shrink: 0;
  min-width: 26px;
  text-align: right;
}
.pool-goto {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 11px;
  cursor: pointer;
  padding: 2px 4px;
  flex-shrink: 0;
}
.pool-goto:hover {
  text-decoration: underline;
}
/* 浮层出入场 */
.pool-enter-active,
.pool-leave-active {
  transition: opacity 0.15s, transform 0.15s;
}
.pool-enter-from,
.pool-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
@keyframes pool-rotate {
  to { transform: rotate(360deg); }
}
@keyframes pool-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}
@media (prefers-reduced-motion: reduce) {
  .pool-spin,
  .pool-dot.run {
    animation: none;
  }
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
  flex: 1;
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
  justify-content: center;
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  border: 1px solid var(--border-strong);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 12px;
  padding: 0;
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
.settings-link {
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
.settings-link:hover {
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
