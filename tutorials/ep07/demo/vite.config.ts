import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5187,
    strictPort: true,
  },
  build: {
    target: "es2021",
    outDir: "dist",
  },
});
