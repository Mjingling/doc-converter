<template>
  <div v-if="running" class="task-progress">
    <!-- 不确定态（单文件操作）：流动动画条 -->
    <div v-if="indeterminate" class="bar-indeterminate"><span /></div>
    <!-- 确定态（多文件批量）：真实百分比 -->
    <NProgress
      v-else
      type="line"
      :percentage="progress ?? 0"
      :processing="true"
      :show-indicator="false"
      color="var(--accent)"
    />
    <p v-if="label" class="progress-label">{{ label }}</p>
  </div>
</template>

<script setup lang="ts">
import { NProgress } from "naive-ui";

defineProps<{
  /** 是否执行中（false 时整个区块不渲染） */
  running: boolean;
  /** 0-100 百分比（确定态使用） */
  progress?: number;
  /** 不确定态动画（单文件操作） */
  indeterminate?: boolean;
  /** 进度条下方文案，如「合并中…」或「处理中 3/10」 */
  label?: string;
}>();
</script>

<style scoped>
.task-progress {
  margin-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.bar-indeterminate {
  position: relative;
  height: 6px;
  border-radius: 3px;
  background: var(--bg-active);
  overflow: hidden;
}
.bar-indeterminate span {
  position: absolute;
  top: 0;
  left: -35%;
  height: 100%;
  width: 35%;
  border-radius: 3px;
  background: var(--accent);
  animation: task-progress-slide 1.2s ease-in-out infinite;
}
@keyframes task-progress-slide {
  0% {
    left: -35%;
  }
  100% {
    left: 100%;
  }
}
.progress-label {
  margin: 0;
  font-size: 13px;
  color: var(--text-sub);
}
</style>
