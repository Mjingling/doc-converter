<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("batchRename.title") }}</h2>
      <p>{{ t("batchRename.subtitle") }}</p>
    </div>

    <div class="upload-zone" @click="pickFiles" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="files.length === 0" class="zone-empty">
        <NIcon :component="TextOutline" :size="34" color="#722ed1" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("batchRename.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="TextOutline" :size="20" color="#722ed1" />
        <span>{{ t("batchRename.added", { n: files.length }) }}</span>
        <button class="clear-btn" @click.stop="clearFiles">&times;</button>
      </div>
    </div>

    <div v-if="files.length > 0" class="form">
      <div class="field">
        <label>{{ t("batchRename.patternLabel") }}</label>
        <NInput v-model:value="pattern" :placeholder="t('batchRename.patternPlaceholder')" />
        <span class="hint">{{ t("batchRename.patternHint") }}</span>
      </div>

      <!-- AI 智能命名：用自然语言描述命名规则，由 AI 生成名称 -->
      <div class="ai-block">
        <div class="ai-title">
          <NIcon :component="SparklesOutline" :size="15" color="#722ed1" />
          <span>{{ t("batchRename.aiTitle") }}</span>
          <span class="ai-tag">{{ t("batchRename.aiTag") }}</span>
        </div>
        <NInput
          v-model:value="aiDesc"
          type="textarea"
          :rows="2"
          :placeholder="t('batchRename.aiDescPlaceholder')"
          :disabled="aiRunning"
        />
        <div class="ai-row">
          <span class="hint">{{ t("batchRename.aiHint") }}</span>
          <button class="ai-btn" :disabled="!aiDesc.trim() || aiRunning" @click="generateAiNames">
            <NIcon :component="SparklesOutline" :size="15" />
            {{ aiRunning ? t("batchRename.aiGenerating") : t("batchRename.aiGenerate") }}
          </button>
        </div>
      </div>

      <div v-if="aiNames" class="field">
        <label>{{ t("batchRename.aiNamesLabel") }}</label>
        <div class="preview-list">
          <div v-for="(f, i) in files" :key="i" class="preview-item ai-edit">
            <span class="old-name" :title="f">{{ f.split(/[/\\]/).pop() }}</span>
            <span class="arrow">&rarr;</span>
            <NInput v-model:value="aiNames[i]" size="small" class="ai-name-input" />
          </div>
        </div>
        <span class="hint">{{ t("batchRename.aiEditHint") }}</span>
      </div>

      <div class="field">
        <label>{{ t("batchRename.previewLabel") }}</label>
        <div class="preview-list">
          <div v-for="(f, i) in files" :key="i" class="preview-item">
            <span class="old-name" :title="f">{{ f.split(/[/\\]/).pop() }}</span>
            <span class="arrow">&rarr;</span>
            <span class="new-name">{{ getNewName(f, i) }}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="action-row">
      <span class="hint">{{ t("batchRename.hint") }}</span>
      <button class="cta" :disabled="files.length === 0 || (!pattern && !aiNames) || running || aiRunning" @click="run">
        <NIcon :component="TextOutline" :size="17" />
        {{ running ? t("common.converting") : t("batchRename.cta") }}
      </button>
    </div>

    <div v-if="results.length" class="results">
      <p class="result-title">{{ t("batchRename.resultTitle", { ok: results.filter(r => r.ok).length, fail: results.filter(r => !r.ok).length }) }}</p>
      <div v-for="(r, i) in results" :key="i" class="result-item" :class="{ ok: r.ok, fail: !r.ok }">
        <span class="result-path">{{ r.old_path.split(/[/\\]/).pop() }} &rarr; {{ r.new_path.split(/[/\\]/).pop() }}</span>
        <span v-if="!r.ok" class="result-err">{{ r.error }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon, NInput, NButton } from "naive-ui";
import { useI18n } from "vue-i18n";
import { TextOutline, SparklesOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { batchRename } from "../api";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";
import { chat } from "../ai";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();
const settings = useSettingsStore();

const files = ref<string[]>([]);
const pattern = ref("");
const results = ref<{ old_path: string; new_path: string; ok: boolean; error: string | null }[]>([]);
const running = ref(false);

/** AI 智能命名：描述与生成结果（null = 未生成） */
const aiDesc = ref("");
const aiNames = ref<string[] | null>(null);
const aiRunning = ref(false);

function getNewName(path: string, index: number): string {
  const base = path.split(/[/\\]/).pop() || path;
  // AI 模式：直接使用生成结果（含扩展名）
  if (aiNames.value && aiNames.value[index]) return aiNames.value[index];
  if (!pattern.value) return base;
  const ext = extOf(path);
  const pad = String(index + 1).padStart(String(files.value.length).length, "0");
  return pattern.value.replace("{n}", pad).replace("{ext}", ext) + "." + ext;
}

function handleFiles(paths: string[]) {
  if (paths.length === 0) return;
  files.value = paths;
  results.value = [];
  aiNames.value = null;
}

function clearFiles() {
  files.value = [];
  pattern.value = "";
  results.value = [];
  aiDesc.value = "";
  aiNames.value = null;
}

async function pickFiles() {
  const sel = await open({ multiple: true });
  if (sel) handleFiles(Array.isArray(sel) ? sel : [sel]);
}

function onDrop(e: DragEvent) {
  const paths = Array.from(e.dataTransfer?.files || []).map(f => (f as any).path);
  handleFiles(paths);
}

async function run() {
  if (files.value.length === 0 || (!pattern.value && !aiNames.value)) return;
  running.value = true;
  const items: [string, string][] = files.value.map((f, i) => [f, getNewName(f, i)]);
  try {
    const res = await batchRename(items);
    results.value = res;
    const ok = res.filter(r => r.ok).length;
    history.add({
      kind: "batchRename",
      name: t("batchRename.resultName", { n: files.value.length }),
      inputs: files.value,
      outputs: res.filter(r => r.ok).map(r => r.new_path),
      ok: ok === res.length,
    });
    if (ok === res.length) {
      message.success(t("batchRename.success", { n: ok }));
    } else {
      message.warning(t("batchRename.partial", { ok, fail: res.length - ok }));
    }
  } catch (e: any) {
    message.error(t("batchRename.fail", { err: e }));
  } finally {
    running.value = false;
  }
}

/** 安全提取扩展名（无扩展名时返回空字符串，避免全名被当作 ext） */
function extOf(path: string): string {
  const base = path.split(/[/\\]/).pop() || "";
  const i = base.lastIndexOf(".");
  return i > 0 ? base.slice(i + 1) : "";
}

/** 名称清洗：去掉非法字符，保证文件系统安全 */
function sanitizeName(name: string): string {
  return name.replace(/[\\/:*?"<>|]/g, "").trim().slice(0, 200);
}

/** 重名去重：同名追加 _1/_2 后缀 */
function dedupe(names: string[]): string[] {
  const seen = new Map<string, number>();
  return names.map((n) => {
    const key = n.toLowerCase();
    const count = seen.get(key) ?? 0;
    seen.set(key, count + 1);
    return count === 0 ? n : `${n}_${count}`;
  });
}

/** 从 AI 回复中提取 JSON 数组（兼容 ```json 代码块包裹） */
function parseNamesReply(reply: string): string[] | null {
  const cleaned = reply
    .replace(/```json|```/g, "")
    .trim()
    .replace(/^[^\[]*/, "")
    .replace(/[^\]]*$/, "");
  try {
    const arr = JSON.parse(cleaned);
    if (!Array.isArray(arr)) return null;
    return arr.map((s) => (typeof s === "string" ? s : "")).map(sanitizeName);
  } catch {
    return null;
  }
}

/** AI 智能命名：chat 生成新文件名列表（不含扩展名） */
async function generateAiNames() {
  if (files.value.length === 0 || !aiDesc.value.trim() || aiRunning.value) return;
  aiRunning.value = true;
  const items = files.value.map((f, i) => `${i + 1}. ${f.split(/[/\\]/).pop() || f}`).join("\n");
  try {
    const reply = await chat([
      {
        role: "system",
        content:
          "你是文件批量重命名助手。根据用户要求为每个文件生成新文件名（不含扩展名）。" +
          `输出 JSON 数组（如 [\"新名1\", \"新名2\"]），数量必须与输入一致，共 ${files.value.length} 个。` +
          "名称不能包含 \\ / : * ? \" < > | 字符，只输出 JSON，不要任何其他内容。",
      },
      {
        role: "user",
        content: `用户要求：${aiDesc.value.trim()}\n\n文件列表：\n${items}`,
      },
    ]);
    const names = parseNamesReply(reply);
    if (!names || names.length !== files.value.length) {
      message.error(t("batchRename.aiCountMismatch"));
      return;
    }
    // 空名回退原文件名（去扩展名），再全局去重
    const finalNames = names.map((n, i) => {
      if (n) return n;
      const base = files.value[i].split(/[/\\]/).pop() || "";
      return base.replace(/\.[^.]+$/, "");
    });
    aiNames.value = dedupe(finalNames).map((n, i) => {
      const ext = extOf(files.value[i]);
      return `${n}.${ext}`;
    });
    message.success(t("batchRename.aiDone", { n: aiNames.value.length }));
  } catch (e: any) {
    const msg = String(e || "");
    if (settings.ai.mode === "local") {
      message.error(t("batchRename.aiFail", { err: "本地模型未下载或加载失败，请在设置 → AI 能力中下载本地模型" }));
    } else if (msg.includes("云端") || !settings.ai.cloud.baseUrl) {
      message.error(t("batchRename.aiCloudUnset"));
    } else {
      message.error(t("batchRename.aiFail", { err: msg }));
    }
  } finally {
    aiRunning.value = false;
  }
}

defineExpose({ handleDrop: handleFiles });
</script>

<style scoped>
.panel { background: var(--bg-panel); border-radius: 14px; padding: 30px; box-shadow: 0 1px 3px var(--shadow); }
.panel-head h2 { margin: 0; font-size: 22px; color: var(--text-main); }
.panel-head p { margin: 6px 0 0; font-size: 13px; color: var(--text-muted); }
.upload-zone { margin-top: 18px; border: 2px dashed var(--border-dash); border-radius: 14px; padding: 28px; text-align: center; cursor: pointer; transition: all 0.2s; }
.upload-zone:hover { border-color: var(--accent); background: var(--accent-soft); }
.zone-empty { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.zone-main { margin: 0; font-size: 14px; color: var(--text-sub); }
.zone-main .link { color: var(--accent); cursor: pointer; }
.zone-sub { margin: 0; font-size: 12px; color: var(--text-faint); }
.zone-filled { display: flex; align-items: center; justify-content: center; gap: 10px; font-size: 14px; color: var(--text-sub); }
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.form { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.field { display: flex; flex-direction: column; gap: 8px; }
.field label { font-size: 13px; color: var(--text-sub); }
.ai-block { display: flex; flex-direction: column; gap: 8px; padding: 14px; border: 1px dashed var(--border-dash); border-radius: 10px; background: var(--accent-soft); }
.ai-title { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 600; color: var(--text-body); }
.ai-tag { font-size: 11px; font-weight: 400; color: var(--accent); border: 1px solid var(--accent); border-radius: 10px; padding: 0 8px; }
.ai-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
.ai-btn { display: flex; align-items: center; gap: 6px; border: none; background: var(--accent); color: var(--cta-text); font-size: 13px; font-weight: 600; padding: 8px 18px; border-radius: 7px; cursor: pointer; transition: opacity 0.15s; }
.ai-btn:hover:not(:disabled) { opacity: 0.85; }
.ai-btn:disabled { background: var(--cta-disabled); cursor: not-allowed; }
.ai-edit { gap: 8px; }
.ai-name-input { flex: 1; min-width: 0; }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
.preview-list { max-height: 240px; overflow-y: auto; border: 1px solid var(--border-soft); border-radius: 8px; padding: 4px 8px; }
.preview-item { display: flex; align-items: center; gap: 10px; padding: 4px 0; font-size: 12px; border-bottom: 1px solid var(--border-soft); }
.preview-item:last-child { border-bottom: none; }
.old-name { color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.arrow { color: var(--text-faint); flex-shrink: 0; }
.new-name { color: var(--accent); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
.results { margin-top: 16px; }
.result-title { font-size: 14px; font-weight: 600; color: var(--green); margin-bottom: 8px; }
.result-item { display: flex; align-items: center; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--border-soft); font-size: 12px; }
.result-item.ok { color: var(--text-sub); }
.result-item.fail { color: var(--red); }
.result-path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.result-err { font-size: 11px; color: var(--red); flex-shrink: 0; margin-left: 8px; }
</style>
