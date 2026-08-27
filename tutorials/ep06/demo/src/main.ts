import { createApp } from "vue";
import MainWindow from "./MainWindow.vue";
import PetWindow from "./PetWindow.vue";

/**
 * 双窗口共用一份前端代码，靠 URL query 分流：
 * - 主窗口：index.html            → MainWindow
 * - 宠物窗口：index.html?window=pet → PetWindow
 *
 * 宠物窗口的 URL 由 Rust 侧 WebviewWindowBuilder 指定（见 main.rs）。
 */
const isPet = new URLSearchParams(location.search).get("window") === "pet";

createApp(isPet ? PetWindow : MainWindow).mount("#app");
