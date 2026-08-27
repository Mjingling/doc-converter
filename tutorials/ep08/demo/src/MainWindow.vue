<template>
  <main class="page" :class="theme">
    <header>
      <div>
        <h1>{{ t("app.title") }}</h1>
        <p class="sub">{{ t("app.subtitle") }}</p>
      </div>
      <div class="toolbar">
        <button @click="toggleLang">{{ t("app.lang") }}</button>
        <button @click="toggleTheme">{{ t("app.theme") }}</button>
      </div>
    </header>

    <div class="actions">
      <button :disabled="running" class="primary" @click="runTask(true)">
        {{ t("app.runSuccess") }}
      </button>
      <button :disabled="running" class="danger" @click="runTask(false)">
        {{ t("app.runFail") }}
      </button>
    </div>

    <div v-if="running" class="progress">
      <div class="bar" :style="{ width: progress + '%' }"></div>
    </div>
    <p v-if="status" class="status">{{ status }}</p>
  </main>
</template>

<script setup lang="ts">
import { onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { emitPetProgress } from "./petProgress";

const { t, locale } = useI18n();

// ── 主题：一个 class 切换整套 CSS 变量 ───────────────────
const theme = ref<"light" | "dark">("light");
function toggleTheme() {
  theme.value = theme.value === "light" ? "dark" : "light";
  // 主题同时作用于宠物窗口？——教学版只管自己；
  // 成品里主题存 tauri-plugin-store，宠物窗口启动时读取同一份
}

// ── 语言：直接改 i18n 的 locale，全组件响应式刷新 ────────
function toggleLang() {
  locale.value = locale.value === "zh-CN" ? "en-US" : "zh-CN";
}

// ── EP06 的模拟任务（台词换成 t()）──────────────────────
const running = ref(false);
const progress = ref(0);
const status = ref("");
let timer = 0;

function runTask(succeed: boolean) {
  running.value = true;
  progress.value = 0;
  status.value = t("app.running");
  void emitPetProgress({ phase: "start" });

  timer = window.setInterval(() => {
    progress.value = Math.min(100, progress.value + 4 + Math.random() * 8);
    void emitPetProgress({ phase: "tick", progress: progress.value });
    if (progress.value >= 100) {
      clearInterval(timer);
      running.value = false;
      if (succeed) {
        status.value = t("app.done");
        void emitPetProgress({ phase: "done", name: t("app.title") });
      } else {
        status.value = t("app.failed");
        void emitPetProgress({ phase: "error", name: t("app.title") });
      }
    }
  }, 200);
}

onUnmounted(() => clearInterval(timer));
</script>

<style scoped>
/* 主题 = 两套 CSS 变量，组件样式全部引用变量，一处切换全局生效 */
.page {
  --bg: #f5f6f8;
  --fg: #1a1a1a;
  --card: #ffffff;
  --border: #e3e5e8;
  --accent: #2080f0;
}
.page.dark {
  --bg: #17181c;
  --fg: #e6e8eb;
  --card: #232428;
  --border: #33353a;
  --accent: #4d9fff;
}

.page {
  min-height: 100vh;
  box-sizing: border-box;
  padding: 28px 24px;
  background: var(--bg);
  color: var(--fg);
  font-family: -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif;
  transition: background 0.25s, color 0.25s;
}
header { display: flex; justify-content: space-between; align-items: flex-start; }
h1 { margin: 0; font-size: 20px; }
.sub { margin: 4px 0 0; font-size: 12px; opacity: 0.6; }
.toolbar { display: flex; gap: 8px; }
button {
  padding: 8px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--card);
  color: var(--fg);
  font-size: 13px;
  cursor: pointer;
}
.actions { display: flex; gap: 10px; margin-top: 22px; }
button.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
button.danger { background: #d03050; border-color: #d03050; color: #fff; }
button:disabled { opacity: 0.55; cursor: not-allowed; }
.progress {
  margin-top: 20px;
  height: 8px;
  border-radius: 4px;
  background: var(--border);
  overflow: hidden;
}
.bar { height: 100%; background: var(--accent); transition: width 0.2s linear; }
.status { margin-top: 14px; font-size: 13px; }
</style>
