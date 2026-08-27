<template>
  <main class="page">
    <h1>文档工具箱 <small>EP05 · Rust 引擎</small></h1>

    <!-- 合并 PDF -->
    <section class="card">
      <h2>合并 PDF</h2>
      <p class="hint">每行一个 PDF 路径（教程版先用文本输入，成品里是拖拽 + 文件对话框）</p>
      <textarea v-model="mergeInputsText" rows="3" placeholder="/Users/xxx/a.pdf&#10;/Users/xxx/b.pdf"></textarea>
      <input v-model="mergeOutput" placeholder="输出路径，如 /Users/xxx/merged.pdf" />
      <button :disabled="busy" @click="doMerge">{{ busy ? "处理中…" : "开始合并" }}</button>
    </section>

    <!-- 压缩 PDF -->
    <section class="card">
      <h2>压缩 PDF</h2>
      <input v-model="compressInput" placeholder="输入 PDF 路径" />
      <input v-model="compressOutput" placeholder="输出路径，如 /Users/xxx/small.pdf" />
      <button :disabled="busy" @click="doCompress">{{ busy ? "处理中…" : "开始压缩" }}</button>
    </section>

    <!-- 结果 -->
    <section v-if="result" class="result" :class="{ error: isError }">{{ result }}</section>
  </main>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const mergeInputsText = ref("");
const mergeOutput = ref("");
const compressInput = ref("");
const compressOutput = ref("");
const busy = ref(false);
const result = ref("");
const isError = ref(false);

function show(ok: boolean, msg: string) {
  isError.value = !ok;
  result.value = msg;
}

async function doMerge() {
  const inputs = mergeInputsText.value
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  if (inputs.length === 0 || !mergeOutput.value.trim()) {
    show(false, "请填写至少一个输入路径和输出路径");
    return;
  }
  busy.value = true;
  try {
    // invoke 的泛型参数 = Rust 命令的返回类型
    const msg = await invoke<string>("pdf_merge", { inputs, output: mergeOutput.value.trim() });
    show(true, msg);
  } catch (e) {
    show(false, String(e));
  } finally {
    busy.value = false;
  }
}

async function doCompress() {
  if (!compressInput.value.trim() || !compressOutput.value.trim()) {
    show(false, "请填写输入路径和输出路径");
    return;
  }
  busy.value = true;
  try {
    const msg = await invoke<string>("pdf_compress", {
      input: compressInput.value.trim(),
      output: compressOutput.value.trim(),
    });
    show(true, msg);
  } catch (e) {
    show(false, String(e));
  } finally {
    busy.value = false;
  }
}
</script>

<style scoped>
.page {
  max-width: 640px;
  margin: 0 auto;
  padding: 32px 24px;
  font-family: -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif;
  color: #1a1a1a;
}
h1 small { font-size: 13px; color: #8a8f98; font-weight: 400; }
.card {
  margin-top: 20px;
  padding: 16px;
  border: 1px solid #e3e5e8;
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.card h2 { margin: 0; font-size: 16px; }
.hint { margin: 0; font-size: 12px; color: #8a8f98; }
input, textarea {
  padding: 8px 10px;
  border: 1px solid #d5d8dc;
  border-radius: 8px;
  font-size: 13px;
  resize: vertical;
}
button {
  align-self: flex-start;
  padding: 8px 18px;
  border: none;
  border-radius: 8px;
  background: #2080f0;
  color: #fff;
  font-size: 14px;
  cursor: pointer;
}
button:disabled { opacity: 0.6; cursor: not-allowed; }
.result {
  margin-top: 20px;
  padding: 12px 14px;
  border-radius: 8px;
  font-size: 13px;
  background: #e8f5e9;
  color: #2e7d32;
}
.result.error { background: #fdecea; color: #c62828; }
</style>
