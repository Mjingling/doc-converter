<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  NConfigProvider, NDialogProvider, NMessageProvider,
  zhCN, dateZhCN, enUS, dateEnUS, jaJP, dateJaJP, koKR, dateKoKR,
  darkTheme,
} from "naive-ui";
import { useI18n } from "vue-i18n";
import Home from "./views/Home.vue";
import { useSettingsStore } from "./stores/settings";
import { resolveSystemLocale } from "./i18n";
import type { LocaleCode } from "./i18n";

const settings = useSettingsStore();
const { locale: i18nLocale } = useI18n();

/* ---------- 主题解析 ---------- */
const systemDark = ref(
  typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches
);
const mq = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
function onMqChange(e: MediaQueryListEvent) {
  systemDark.value = e.matches;
}
onMounted(() => mq?.addEventListener("change", onMqChange));
onUnmounted(() => mq?.removeEventListener("change", onMqChange));

/** 实际生效的主题（跟随系统时解析系统设置） */
const resolvedDark = computed(() =>
  settings.theme === "system" ? systemDark.value : settings.theme === "dark"
);

watch(resolvedDark, (dark) => {
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}, { immediate: true });

/* ---------- 语言解析 ---------- */
/** 实际生效的语言（跟随系统时解析系统语言） */
const resolvedLocale = computed<LocaleCode>(() =>
  settings.locale === "system" ? resolveSystemLocale() : settings.locale
);
watch(resolvedLocale, (code) => {
  i18nLocale.value = code;
}, { immediate: true });

/** naive-ui 组件语言包（按当前语言切换） */
const naiveLocale = computed(() => {
  switch (resolvedLocale.value) {
    case "en-US": return { locale: enUS, dateLocale: dateEnUS };
    case "ja-JP": return { locale: jaJP, dateLocale: dateJaJP };
    case "ko-KR": return { locale: koKR, dateLocale: dateKoKR };
    default: return { locale: zhCN, dateLocale: dateZhCN };
  }
});
</script>

<template>
  <NConfigProvider
    :theme="resolvedDark ? darkTheme : null"
    :locale="naiveLocale.locale"
    :date-locale="naiveLocale.dateLocale"
    style="height: 100%"
  >
    <NDialogProvider>
      <NMessageProvider>
        <Home />
      </NMessageProvider>
    </NDialogProvider>
  </NConfigProvider>
</template>

<style>
/* 全局 CSS 变量：浅色主题（默认） */
:root,
html[data-theme="light"] {
  --bg-page: #f8f9fa;
  --bg-panel: #ffffff;
  --bg-hover: #f5f6f7;
  --bg-active: #e6e8eb;
  --bg-input: #fafbfc;
  --bg-tag: #f5f6f7;
  --text-main: #1a1a1a;
  --text-body: #333333;
  --text-sub: #666666;
  --text-muted: #999999;
  --text-faint: #b0b3b8;
  --border: #eeeeee;
  --border-soft: #f5f5f5;
  --border-strong: #e5e6e8;
  --border-dash: #d9d9d9;
  --cta-bg: #1a1a1a;
  --cta-text: #ffffff;
  --cta-disabled: #d0d0d0;
  --accent: #2080f0;
  --accent-soft: #f7fbff;
  --green: #18a058;
  --green-soft: #ecf7f0;
  --red: #e6494c;
  --red-soft: #fdeeee;
  --orange: #e6a23c;
  --orange-soft: #fdf6ec;
  --shadow: rgba(0, 0, 0, 0.04);
}

/* 深色主题 */
html[data-theme="dark"] {
  --bg-page: #17171b;
  --bg-panel: #232327;
  --bg-hover: #2e2e35;
  --bg-active: #38383f;
  --bg-input: #2b2b31;
  --bg-tag: #2e2e35;
  --text-main: #f2f3f5;
  --text-body: #d9dae0;
  --text-sub: #a9abb2;
  --text-muted: #787a82;
  --text-faint: #5f6168;
  --border: #33333a;
  --border-soft: #2b2b31;
  --border-strong: #44444c;
  --border-dash: #4a4a52;
  --cta-bg: #f2f3f5;
  --cta-text: #17171b;
  --cta-disabled: #48484f;
  --accent: #4c9aff;
  --accent-soft: #1d2c3d;
  --green: #4fc08d;
  --green-soft: #1c3327;
  --red: #f06a6d;
  --red-soft: #3a2325;
  --orange: #e6a23c;
  --orange-soft: #3a2f1c;
  --shadow: rgba(0, 0, 0, 0.3);
}

html,
body,
#app {
  height: 100%;
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, "PingFang SC", "Helvetica Neue",
    "Microsoft YaHei", sans-serif;
  background: var(--bg-page);
  color: var(--text-body);
}

/* 深色模式下滚动条与系统一致，naive-ui 组件由 NConfigProvider 统一处理 */
</style>
