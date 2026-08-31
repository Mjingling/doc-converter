<template>
  <header
    v-if="!fullscreen"
    class="titlebar"
    data-tauri-drag-region
  >
    <!-- macOS：红绿灯留白 + 应用名 -->
    <template v-if="isMac">
      <div class="traffic-pad" data-tauri-drag-region></div>
      <div class="title" data-tauri-drag-region>{{ t("app.name") }}</div>
    </template>
    <!-- Windows：占位拖拽区（右上角操作入口已迁至左下角设置菜单） -->
    <div v-else class="win-spacer" data-tauri-drag-region></div>
  </header>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isMac } from "../utils/platform";

const { t } = useI18n();
const fullscreen = ref(false);

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

/* Windows：占位拖拽区（保持整条标题栏可拖拽） */
.win-spacer {
  flex: 1;
  height: 100%;
}
</style>
