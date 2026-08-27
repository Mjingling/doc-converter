<template>
  <main class="page">
    <h1>文档工具箱 <small>EP06 · 双窗口联动</small></h1>
    <p class="hint">点击按钮模拟一个耗时任务，盯着右下角的宠物看它的反应。</p>

    <div class="actions">
      <button :disabled="running" @click="runTask(true)">▶ 模拟任务（成功）</button>
      <button :disabled="running" class="danger" @click="runTask(false)">▶ 模拟任务（失败）</button>
    </div>

    <div v-if="running" class="progress">
      <div class="bar" :style="{ width: progress + '%' }"></div>
    </div>
    <p v-if="status" class="status">{{ status }}</p>
  </main>
</template>

<script setup lang="ts">
import { onUnmounted, ref } from "vue";
import { emitPetProgress } from "./petProgress";

const running = ref(false);
const progress = ref(0);
const status = ref("");
let timer = 0;

/**
 * 模拟一个 3~5 秒的文档处理任务。
 * 换成真实任务时（比如 invoke("pdf_merge")），只需在关键节点
 * 调用同样的 emitPetProgress —— 宠物侧一行都不用改。
 */
function runTask(succeed: boolean) {
  running.value = true;
  progress.value = 0;
  status.value = "任务进行中…";
  void emitPetProgress({ phase: "start" });

  timer = window.setInterval(() => {
    progress.value = Math.min(100, progress.value + 4 + Math.random() * 8);
    void emitPetProgress({ phase: "tick", progress: progress.value });

    if (progress.value >= 100) {
      clearInterval(timer);
      running.value = false;
      if (succeed) {
        status.value = "任务完成 ✅";
        void emitPetProgress({ phase: "done", name: "模拟合并" });
      } else {
        status.value = "任务失败 ❌（故意的，看看宠物怎么安慰你）";
        void emitPetProgress({ phase: "error", name: "模拟合并" });
      }
    }
  }, 200);
}

onUnmounted(() => clearInterval(timer));
</script>

<style scoped>
.page {
  max-width: 520px;
  margin: 0 auto;
  padding: 36px 24px;
  font-family: -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif;
  color: #1a1a1a;
}
h1 small { font-size: 13px; color: #8a8f98; font-weight: 400; }
.hint { font-size: 13px; color: #8a8f98; }
.actions { display: flex; gap: 10px; margin-top: 16px; }
button {
  padding: 9px 18px;
  border: none;
  border-radius: 8px;
  background: #2080f0;
  color: #fff;
  font-size: 14px;
  cursor: pointer;
}
button.danger { background: #d03050; }
button:disabled { opacity: 0.6; cursor: not-allowed; }
.progress {
  margin-top: 20px;
  height: 8px;
  border-radius: 4px;
  background: #eef0f3;
  overflow: hidden;
}
.bar {
  height: 100%;
  background: #2080f0;
  transition: width 0.2s linear;
}
.status { margin-top: 14px; font-size: 13px; }
</style>
