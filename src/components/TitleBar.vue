<template>
  <header
    v-if="isMac && !fullscreen"
    class="titlebar"
    data-tauri-drag-region
  >
    <!-- 左侧给系统红绿灯按钮留白 -->
    <div class="traffic-pad" data-tauri-drag-region></div>
    <div class="title" data-tauri-drag-region>{{ t("app.name") }}</div>
    <div class="actions">
      <button
        class="icon-btn"
        :title="t('common.toggleTheme')"
        :aria-label="t('common.toggleTheme')"
        @click="toggleTheme"
      >
        <NIcon :component="isDarkNow ? SunnyOutline : MoonOutline" :size="16" />
      </button>
      <button
        class="icon-btn"
        :title="t('common.settings')"
        :aria-label="t('common.settings')"
        @click="$emit('open-settings')"
      >
        <NIcon :component="SettingsOutline" :size="16" />
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { NIcon } from "naive-ui";
import { useI18n } from "vue-i18n";
import { SettingsOutline, SunnyOutline, MoonOutline } from "@vicons/ionicons5";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsStore } from "../stores/settings";
import { isMac } from "../utils/platform";

defineEmits<{ (e: "open-settings"): void }>();

const { t } = useI18n();
const settings = useSettingsStore();
const fullscreen = ref(false);

/** 当前是否深色（含跟随系统时解析系统偏好） */
const isDarkNow = computed(() =>
  settings.theme === "dark" ||
  (settings.theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches)
);

/** 快速切换浅色 / 深色主题（从跟随系统切到显式模式） */
function toggleTheme() {
  settings.setTheme(isDarkNow.value ? "light" : "dark");
}

/* macOS 全屏时系统自带标题栏，隐藏自定义标题栏避免重复
 * 当前 @tauri-apps/api 无全屏事件，通过 resize 事件查询全屏状态 */
let unResized: (() => void) | null = null;
onMounted(async () => {
  try {
    const win = getCurrentWindow();
    unResized = await win.onResized(async () => {
      fullscreen.value = await win.isFullscreen();
    });
  } catch {
    // 非 Tauri 环境（浏览器预览）忽略
  }
});
onUnmounted(() => {
  unResized?.();
});
</script>

<style scoped>
.titlebar {
  height: 40px;
  flex: none;
  display: flex;
  align-items: center;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
  -webkit-user-select: none;
  user-select: none;
}

.traffic-pad {
  width: 80px;
  height: 100%;
  flex: none;
}

.title {
  flex: 1;
  text-align: center;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-sub);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.actions {
  width: 92px;
  height: 100%;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  padding-right: 12px;
}

.icon-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-sub);
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-main);
}
</style>
