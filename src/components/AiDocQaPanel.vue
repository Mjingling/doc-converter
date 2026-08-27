<template>
  <div class="docqa-panel">
    <!-- 云端配置提示 -->
    <div v-if="!cloudReady" class="cloud-warn">
      <NIcon :component="CloudOfflineOutline" :size="16" />
      <span>{{ t("aiAssistant.needCloud") }}</span>
    </div>

    <!-- 文档区：添加 + 列表（索引状态/进度/删除） -->
    <div class="doc-section">
      <div class="doc-toolbar">
        <button class="add-btn" @click="pickDocs">
          <NIcon :component="AddOutline" :size="15" />
          {{ t("docQa.addDocs") }}
        </button>
        <span class="doc-hint">{{ t("docQa.docHint") }}</span>
        <span class="spacer" />
        <button v-if="rag.msgs.length" class="link-btn" @click="rag.clearMsgs()">{{ t("docQa.clearChat") }}</button>
      </div>
      <div v-if="rag.docs.length" class="doc-list">
        <div v-for="d in rag.docs" :key="d.id" class="doc-chip" :class="d.status" :title="d.path">
          <NIcon :component="DocumentTextOutline" :size="15" class="doc-icon" />
          <span class="doc-name">{{ d.name }}</span>
          <span v-if="d.status === 'indexing'" class="doc-state">{{ t("docQa.indexing", { done: d.batchDone, total: d.batchTotal }) }}</span>
          <span v-else-if="d.status === 'ready'" class="doc-state ok">{{ t("docQa.ready", { n: d.index?.chunks.length ?? 0 }) }}</span>
          <span v-else class="doc-state fail">{{ t("docQa.failed") }}</span>
          <button class="doc-del" @click="rag.removeDoc(d.id)">&times;</button>
        </div>
      </div>
      <div v-else class="doc-empty">{{ t("docQa.noDocs") }}</div>
    </div>

    <!-- 消息列表 -->
    <div ref="listEl" class="msg-list">
      <div v-if="rag.msgs.length === 0" class="msg-empty">{{ t("docQa.chatEmpty") }}</div>
      <div v-for="m in rag.msgs" :key="m.id" class="msg-row" :class="m.role">
        <div class="bubble" :class="m.role">
          <span class="who">{{ m.role === "user" ? t("aiAssistant.you") : t("aiAssistant.assistant") }}</span>
          <span class="text">{{ m.content }}</span>
          <!-- 引用来源（可展开：文档名 + 片段摘录） -->
          <div v-if="m.hits && m.hits.length" class="hits">
            <button class="hits-toggle" @click="toggleHits(m.id)">
              {{ hitsOpen.has(m.id) ? t("docQa.hideSources") : t("docQa.showSources", { n: m.hits.length }) }}
            </button>
            <div v-if="hitsOpen.has(m.id)" class="hits-list">
              <div v-for="(h, i) in m.hits" :key="i" class="hit-item">
                <span class="hit-tag">{{ t("docQa.sourceTag", { i: i + 1 }) }} · {{ h.docName }}</span>
                <span class="hit-text">{{ h.text.slice(0, 120) }}{{ h.text.length > 120 ? "…" : "" }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 输入区 -->
    <div class="input-card">
      <textarea
        v-model="input"
        class="chat-input"
        rows="2"
        :placeholder="t('docQa.inputPlaceholder')"
        :disabled="busy"
        spellcheck="false"
        @keydown.enter.exact.prevent="send"
      ></textarea>
      <div class="card-actions">
        <span class="spacer" />
        <button
          class="send-btn"
          :title="busy ? t('aiAssistant.sending') : t('aiAssistant.send')"
          :disabled="!input.trim() || busy || !cloudReady || !rag.readyDocs.length"
          @click="send"
        >
          <NIcon :component="busy ? SyncOutline : SendOutline" :size="16" :class="{ spin: busy }" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { NIcon, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import {
  AddOutline, CloudOfflineOutline, DocumentTextOutline, SendOutline, SyncOutline,
} from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "../stores/settings";
import { useRagStore } from "../stores/rag";
import type { RagDoc } from "../stores/rag";
import { chat } from "../ai";
import { buildIndex, buildQaMessages, searchTopK } from "../ai/rag";
import type { RagDocEntry } from "../ai/rag";
import { extractText } from "../api";
import { extOf } from "../utils/file";

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const rag = useRagStore();

/** 云端配置就绪（问答依赖云端 embed + chat） */
const cloudReady = computed(() => !!(settings.ai.cloud.baseUrl && settings.ai.cloud.apiKey));

/** 与 AiSummaryPanel 一致的文本可提取扩展名 */
const SUPPORTED_EXTS = ["pdf", "docx", "txt", "md", "markdown", "csv", "json", "xml", "html", "htm", "log"];

const input = ref("");
const busy = ref(false);
const listEl = ref<HTMLElement | null>(null);
/** 展开引用来源的消息 id 集合 */
const hitsOpen = ref(new Set<number>());

function toggleHits(id: number) {
  const s = new Set(hitsOpen.value);
  if (s.has(id)) s.delete(id);
  else s.add(id);
  hitsOpen.value = s;
}

function scrollToBottom() {
  nextTick(() => {
    const el = listEl.value;
    if (el) el.scrollTop = el.scrollHeight;
  });
}

async function pickDocs() {
  const sel = await open({ multiple: true });
  if (sel) await handleFiles(Array.isArray(sel) ? sel.map(String) : [String(sel)]);
}

/** 逐个文档：提取文本 → 分块向量化 → 就绪；进度写回 store */
async function handleFiles(paths: string[]) {
  for (const p of paths) {
    if (!SUPPORTED_EXTS.includes(extOf(p).toLowerCase())) {
      message.warning(t("docQa.warnUnsupported", { name: p.split(/[/\\]/).pop() }));
      continue;
    }
    if (rag.docs.some((d) => d.path === p)) continue;
    const doc = rag.addDoc(p);
    void buildDocIndex(doc);
  }
}

async function buildDocIndex(doc: RagDoc) {
  try {
    const text = await extractText(doc.path);
    if (!text.trim()) throw new Error(t("docQa.noText"));
    doc.index = await buildIndex(text, (done, total) => {
      doc.batchDone = done;
      doc.batchTotal = total;
    });
    doc.status = "ready";
  } catch (e: any) {
    doc.status = "failed";
    doc.error = String(e);
    message.error(t("docQa.indexFail", { name: doc.name, err: e }));
  }
}

/** 提问：检索 top-k 片段 → 拼提示词 → chat */
async function send() {
  const q = input.value.trim();
  if (!q || busy.value) return;
  if (!rag.readyDocs.length) {
    message.warning(t("docQa.noReadyDoc"));
    return;
  }
  rag.addMsg({ role: "user", content: q });
  input.value = "";
  busy.value = true;
  scrollToBottom();
  try {
    const entries: RagDocEntry[] = rag.readyDocs.map((d) => ({
      docId: d.id,
      docName: d.name,
      index: d.index!,
    }));
    const hits = await searchTopK(entries, q);
    const reply = await chat(buildQaMessages(hits, q));
    const m = rag.addMsg({ role: "assistant", content: reply.trim() || t("aiAssistant.emptyReply"), hits });
    // 默认展开最新一条回答的引用来源
    hitsOpen.value = new Set([...hitsOpen.value, m.id]);
  } catch (e: any) {
    rag.addMsg({ role: "assistant", content: t("aiAssistant.fail", { err: String(e) }) });
  } finally {
    busy.value = false;
    scrollToBottom();
  }
}

/** 供 Home.vue 拖拽分发：拖入的文档直接加入并建索引 */
defineExpose({ handleDrop: handleFiles });
</script>

<style scoped>
.docqa-panel { display: flex; flex-direction: column; height: 100%; min-height: 0; gap: 12px; }
.cloud-warn { display: flex; align-items: center; gap: 8px; flex-shrink: 0; font-size: 12px; color: var(--orange); background: var(--orange-soft); border-radius: 10px; padding: 10px 14px; }
/* 文档区 */
.doc-section { flex-shrink: 0; border: 1px solid var(--border); border-radius: 12px; background: var(--bg-panel); padding: 12px 14px; display: flex; flex-direction: column; gap: 10px; }
.doc-toolbar { display: flex; align-items: center; gap: 10px; }
.spacer { flex: 1; }
.add-btn { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--border-strong); background: var(--bg-tag); color: var(--accent); font-size: 12px; padding: 6px 12px; border-radius: 8px; cursor: pointer; transition: border-color 0.15s; }
.add-btn:hover { border-color: var(--accent); }
.doc-hint { font-size: 11px; color: var(--text-faint); }
.link-btn { border: none; background: none; color: var(--text-muted); font-size: 12px; cursor: pointer; }
.link-btn:hover { color: var(--accent); }
.doc-list { display: flex; flex-wrap: wrap; gap: 8px; max-height: 96px; overflow-y: auto; }
.doc-empty { font-size: 12px; color: var(--text-faint); }
.doc-chip { display: inline-flex; align-items: center; gap: 6px; background: var(--bg-tag); border: 1px solid var(--border); border-radius: 10px; padding: 5px 8px; max-width: 300px; font-size: 12px; color: var(--text-sub); }
.doc-icon { flex-shrink: 0; color: var(--accent); }
.doc-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 140px; }
.doc-state { font-size: 11px; color: var(--text-faint); white-space: nowrap; }
.doc-state.ok { color: var(--green); }
.doc-state.fail { color: var(--red); }
.doc-del { border: none; background: none; color: var(--text-muted); font-size: 14px; cursor: pointer; padding: 0 2px; line-height: 1; }
.doc-del:hover { color: var(--red); }
/* 消息列表 */
.msg-list { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 10px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: 12px; padding: 14px; }
.msg-empty { margin: auto; text-align: center; color: var(--text-muted); font-size: 13px; }
.msg-row.user { align-self: flex-end; }
.msg-row.assistant { align-self: flex-start; }
.bubble { display: flex; flex-direction: column; gap: 3px; max-width: 82%; border-radius: 12px; padding: 10px 14px; font-size: 13px; line-height: 1.7; word-break: break-word; white-space: pre-wrap; }
.bubble.user { background: var(--accent); color: #fff; border-bottom-right-radius: 4px; }
.bubble.assistant { background: var(--bg-tag); color: var(--text-body); border-bottom-left-radius: 4px; }
.who { font-size: 11px; opacity: 0.75; }
/* 引用来源 */
.hits { margin-top: 6px; }
.hits-toggle { border: none; background: none; color: var(--accent); font-size: 11px; cursor: pointer; padding: 0; }
.hits-list { margin-top: 6px; display: flex; flex-direction: column; gap: 6px; }
.hit-item { border-left: 2px solid var(--accent); padding: 4px 8px; background: var(--bg-page); border-radius: 0 6px 6px 0; }
.hit-tag { display: block; font-size: 10px; color: var(--text-faint); margin-bottom: 2px; }
.hit-text { font-size: 11px; line-height: 1.6; color: var(--text-sub); white-space: pre-wrap; word-break: break-word; }
/* 输入区 */
.input-card { flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; border: 1px solid var(--border-strong); border-radius: 14px; background: var(--bg-input); padding: 12px; transition: border-color 0.15s; }
.input-card:focus-within { border-color: var(--accent); }
.chat-input { width: 100%; border: none; background: transparent; resize: none; outline: none; font: inherit; font-size: 13px; line-height: 1.6; color: var(--text-main); min-height: 40px; max-height: 140px; box-sizing: border-box; }
.chat-input::placeholder { color: var(--text-faint); }
.card-actions { display: flex; align-items: center; gap: 8px; }
.send-btn { display: inline-flex; align-items: center; justify-content: center; width: 36px; height: 36px; border: none; border-radius: 10px; background: var(--cta-bg); color: var(--cta-text); cursor: pointer; flex-shrink: 0; transition: opacity 0.15s; }
.send-btn:hover:not(:disabled) { opacity: 0.85; }
.send-btn:disabled { opacity: 0.45; cursor: not-allowed; }
.send-btn .spin { animation: rotate 1s linear infinite; }
@keyframes rotate { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
