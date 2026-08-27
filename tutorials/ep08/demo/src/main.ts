import { createApp } from "vue";
import { createI18n } from "vue-i18n";
import MainWindow from "./MainWindow.vue";
import PetWindow from "./PetWindow.vue";
import { messages } from "./locales";

/**
 * 语言选择：跟随系统，中文系环境用中文，其余英文。
 * 成品还提供手动切换 + 持久化（tauri-plugin-store）。
 */
const sysLang = navigator.language ?? "en-US";
const locale = sysLang.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";

const i18n = createI18n({
  legacy: false, // Composition API 模式
  locale,
  fallbackLocale: "en-US",
  messages,
});

// EP06 的 query 分流依然有效 —— i18n 对两个窗口一视同仁
const isPet = new URLSearchParams(location.search).get("window") === "pet";

const app = createApp(isPet ? PetWindow : MainWindow);
app.use(i18n);
app.mount("#app");
