<template>
  <!-- 统一的操作结果栏：成功提示 + 打开文件 / 打开所在目录 -->
  <div v-if="outputs.length" class="result-bar">
    <NIcon :component="CheckmarkCircleOutline" :size="18" color="var(--green)" />
    <span class="bar-msg" :title="text">{{ text }}</span>
    <span class="bar-spacer" />
    <NButton size="tiny" secondary type="primary" :title="fileTitle" @click="openFile">
      <template #icon><NIcon :component="OpenOutline" /></template>
      {{ t("common.open") }}
    </NButton>
    <NButton size="tiny" secondary @click="openDir">
      <template #icon><NIcon :component="FolderOpenOutline" /></template>
      {{ t("common.openDir") }}
    </NButton>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NIcon } from "naive-ui";
import { useI18n } from "vue-i18n";
import { CheckmarkCircleOutline, FolderOpenOutline, OpenOutline } from "@vicons/ionicons5";
import { openPath } from "../api";
import { dirOf } from "../utils/file";

/** text: 成功提示文案；outputs: 输出文件绝对路径列表（多文件时“打开文件”打开第一个） */
const props = defineProps<{ text: string; outputs: string[] }>();
const { t } = useI18n();

const fileTitle = computed(() =>
  props.outputs.length > 1 ? `${props.outputs[0].split(/[/\\]/).pop()} ${t("common.etc")}${props.outputs.length}` : props.outputs[0]
);

function openFile() {
  openPath(props.outputs[0]).catch(() => {});
}
function openDir() {
  openPath(dirOf(props.outputs[0])).catch(() => {});
}
</script>

<style scoped>
.result-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 16px;
  padding: 10px 14px;
  background: var(--accent-soft);
  border: 1px solid var(--border);
  border-radius: 10px;
}
.bar-msg {
  flex: 0 1 auto;
  min-width: 0;
  font-size: 13px;
  color: var(--text-sub);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bar-spacer {
  flex: 1;
}
</style>
