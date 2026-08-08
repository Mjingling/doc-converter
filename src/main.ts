import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import i18n from "./i18n";
import { useSettingsStore } from "./stores/settings";
import { useEngineStore } from "./stores/engine";
import { useHistoryStore } from "./stores/history";

async function bootstrap() {
  const app = createApp(App);
  const pinia = createPinia();
  app.use(pinia);

  // 挂载前从本地文件恢复设置与引擎模式（毫秒级，避免界面闪变）
  await useSettingsStore(pinia).hydrate();
  await useEngineStore(pinia).hydrate();
  // 历史记录异步加载（不阻塞启动）
  void useHistoryStore(pinia).hydrate();

  app.use(i18n);
  app.mount("#app");
}

void bootstrap();
