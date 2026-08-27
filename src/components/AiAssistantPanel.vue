<template>
  <div class="assistant-panel">
    <!-- 云端配置提示 -->
    <div v-if="!cloudReady" class="cloud-warn">
      <NIcon :component="CloudOfflineOutline" :size="16" />
      <span>{{ t("aiAssistant.needCloud") }}</span>
    </div>

    <!-- 消息列表 -->
    <div ref="listEl" class="msg-list">
      <div v-if="msgs.length === 0" class="msg-empty">
        <!-- 快捷提示：点击填入输入框（已有附件时直接发送） -->
        <AssistantAvatar size="lg" state="idle" />
        <p class="quick-title">{{ t("aiAssistant.quickTitle") }}</p>
        <p class="quick-sub">{{ t("aiAssistant.quickSub") }}</p>
        <div class="quick-grid">
          <button
            v-for="q in QUICK_PROMPTS"
            :key="q.key"
            class="quick-card"
            :title="t(`aiAssistant.quick.${q.key}`)"
            @click="useQuickPrompt(q.key)"
          >
            <span class="quick-emoji">{{ q.icon }}</span>
            <span class="quick-text">{{ t(`aiAssistant.quick.${q.key}`) }}</span>
          </button>
        </div>
      </div>
      <div v-for="m in msgs" :key="m.id" class="msg-row" :class="m.role">
        <!-- 助手侧行首迷你头像：文本消息静态待机；工具卡片随执行状态换表情 -->
        <AssistantAvatar
          v-if="m.role !== 'user'"
          class="row-avatar"
          size="sm"
          :state="m.role === 'tool' ? toolAvatarState(m) : 'idle'"
          :quiet="m.role !== 'tool' || !m.running"
          track="none"
        />
        <div v-if="m.role === 'tool'" class="tool-card" :class="{ ok: m.ok, fail: m.ok === false }">
          <div class="tool-head">
            <NIcon :component="m.running ? (SyncOutline) : (m.ok === false ? CloseCircleOutline : CheckmarkCircleOutline)" :size="14" :class="{ spin: m.running }" />
            <span class="tool-name">{{ m.toolName }}</span>
            <span v-if="m.running" class="tool-state">{{ t("aiAssistant.toolRunning") }}</span>
            <span v-else class="tool-state">{{ m.ok ? t("aiAssistant.toolOk") : t("aiAssistant.toolFail") }}</span>
          </div>
          <div v-if="m.content" class="tool-body">{{ m.content }}</div>
          <div v-if="m.ok && m.outputs && m.outputs.length" class="tool-actions">
            <button class="tool-open-btn" :title="m.outputs[0]" @click="openPath(m.outputs[0])">{{ t("common.open") }}</button>
            <button class="tool-open-btn" @click="openPath(dirOf(m.outputs[0]))">{{ t("common.openDir") }}</button>
          </div>
        </div>
        <div v-else class="bubble" :class="m.role">
          <!-- 头像已标识身份，助手侧不再显示文字标签；用户侧保留 -->
          <span v-if="m.role === 'user'" class="who">{{ t("aiAssistant.you") }}</span>
          <span class="text">{{ m.content }}</span>
          <!-- 用户消息关联的附件：气泡下方展示，点击打开文件 -->
          <div v-if="m.role === 'user' && m.files?.length" class="msg-files">
            <button
              v-for="f in m.files"
              :key="f"
              class="msg-file-chip"
              :title="f"
              @click="openPath(f)"
            >
              <span class="chip-icon" :style="{ color: fileMeta(f).color }">
                <NIcon :component="fileMeta(f).icon" :size="14" />
              </span>
              <span class="msg-file-name">{{ f.split(/[/\\]/).pop() }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 助手状态气泡：等待模型/执行工具/收尾反馈（迷你头像随状态换表情） -->
      <div v-if="bubbleVisible" class="typing-row">
        <AssistantAvatar size="sm" :state="bubbleState" />
        <div v-if="bubbleState === 'thinking' || bubbleState === 'working'" class="typing-dots">
          <span></span><span></span><span></span>
        </div>
      </div>
    </div>

    <!-- 输入卡片：附件与输入框一体化（点击 + 或拖拽文件到卡片添加附件） -->
    <div
      class="input-card"
      :class="{ 'drag-over': dragOver }"
      @dragover.prevent
      @dragenter.prevent="onDragEnter"
      @dragleave="onDragLeave"
      @drop.prevent="onDrop"
    >
      <div v-if="attached.length" class="attach-list">
        <span v-for="(f, i) in attached" :key="f" class="attach-chip" :title="f">
          <span class="chip-icon" :style="{ color: fileMeta(f).color }">
            <NIcon :component="fileMeta(f).icon" :size="18" />
          </span>
          <span class="chip-text">
            <span class="chip-name">{{ f.split(/[/\\]/).pop() }}</span>
            <span class="chip-ext">{{ (f.split(".").pop() || "").toUpperCase() }}</span>
          </span>
          <button class="chip-close" @click.stop="removeFile(i)">&times;</button>
        </span>
      </div>
      <textarea
        ref="inputEl"
        v-model="input"
        class="chat-input"
        rows="2"
        :placeholder="t('aiAssistant.inputPlaceholder')"
        :disabled="busy"
        spellcheck="false"
        @keydown.enter.exact.prevent="send"
      ></textarea>
      <div class="card-actions">
        <button class="icon-btn" :title="t('aiAssistant.attachHint')" :disabled="busy" @click="pickFiles">
          <NIcon :component="AddOutline" :size="17" />
        </button>
        <button
          class="icon-btn"
          :class="{ active: searchOn }"
          :title="searchOn ? t('aiAssistant.searchOnTip') : t('aiAssistant.searchOffTip')"
          @click="toggleSearch"
        >
          <NIcon :component="GlobeOutline" :size="16" />
        </button>
        <button v-if="msgs.length" class="icon-btn" :title="t('aiAssistant.clear')" @click="clearChat">
          <NIcon :component="TrashOutline" :size="15" />
        </button>
        <span class="actions-spacer" />
        <button
          class="send-btn"
          :title="busy ? t('aiAssistant.sending') : t('aiAssistant.send')"
          :disabled="!input.trim() || busy || !cloudReady"
          @click="send"
        >
          <NIcon :component="busy ? SyncOutline : SendOutline" :size="16" :class="{ spin: busy }" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch, type Component } from "vue";
import { NIcon, useDialog, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { emit } from "@tauri-apps/api/event";
import {
  AddOutline, ArchiveOutline, CheckmarkCircleOutline, CloseCircleOutline, CloudOfflineOutline,
  DocumentOutline, DocumentTextOutline, EaselOutline, FilmOutline, GlobeOutline, GridOutline, ImageOutline,
  MusicalNotesOutline, SendOutline, SyncOutline, TrashOutline,
} from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import AssistantAvatar, { type AvatarState } from "./AssistantAvatar.vue";
import { useSettingsStore } from "../stores/settings";
import { chatWithTools } from "../ai";
import type { ChatMessage, ToolCall, ToolDefinition } from "../ai";
import { AI_TOOLS, executeTool, findTool, getToolDefinitions } from "../ai/tools";
import { searchAvailable } from "../ai/tools/search";
import type { ToolContext, ToolResult } from "../ai/tools";
import { openPath } from "../api";
import { dirOf } from "../utils/file";

const { t } = useI18n();
const message = useMessage();
const dialog = useDialog();
const settings = useSettingsStore();

/** 云端配置就绪（助手依赖云端模型，本地模型不支持工具调用） */
const cloudReady = computed(() => !!(settings.ai.cloud.baseUrl && settings.ai.cloud.apiKey));

/* ---------- 网页搜索快捷开关：直接读写设置中的 provider，与设置页同步 ---------- */
const searchOn = computed(() => settings.ai.search.provider !== "off");

function toggleSearch() {
  const cur = settings.ai.search;
  if (cur.provider !== "off") {
    settings.setAiConfig({ ...settings.ai, search: { ...cur, provider: "off" } });
    message.info(t("aiAssistant.searchOffMsg"));
    return;
  }
  // 开启：优先选已配好密钥的提供商（tavily 免费额度优先）；都不可用则提示去设置里配置
  const next = cur.tavilyKey ? "tavily" : cloudReady.value ? "zhipu" : null;
  if (!next) {
    message.warning(t("aiAssistant.searchNeedConfig"));
    return;
  }
  settings.setAiConfig({ ...settings.ai, search: { ...cur, provider: next } });
  message.success(t("aiAssistant.searchOnMsg"));
}

/** 附件文件（绝对路径，作为工具调用的上下文注入） */
const attached = ref<string[]>([]);
const input = ref("");
const busy = ref(false);
const listEl = ref<HTMLElement | null>(null);

/** 单轮最多工具调用轮数（防止 LLM 死循环） */
const MAX_TOOL_TURNS = 6;

/** 界面消息（user / assistant 气泡 + tool 执行卡片） */
interface UiMsg {
  id: number;
  role: "user" | "assistant" | "tool";
  content: string;
  /** 用户消息发送时携带的附件（气泡下方展示，与该消息关联） */
  files?: string[];
  toolName?: string;
  ok?: boolean;
  running?: boolean;
  /** 工具成功时的输出文件路径（卡片上提供打开入口） */
  outputs?: string[];
}
const msgs = ref<UiMsg[]>([]);
let seq = 0;

/** 工具卡片行首头像表情：执行中 working / 失败 error / 完成 success */
function toolAvatarState(m: UiMsg): AvatarState {
  if (m.running) return "working";
  if (m.ok === false) return "error";
  if (m.ok) return "success";
  return "idle";
}

/** 空状态快捷提示（emoji + 文案 key，点击后填入输入框） */
const QUICK_PROMPTS = [
  { key: "images2pdf", icon: "🖼️" },
  { key: "compress", icon: "📦" },
  { key: "encrypt", icon: "🔐" },
  { key: "watermark", icon: "🖊️" },
  { key: "convert", icon: "🔄" },
  { key: "split", icon: "✂️" },
];
const inputEl = ref<HTMLTextAreaElement | null>(null);

/* ---------- 助手状态气泡：迷你头像随对话阶段切换表情 ---------- */
/** 当前气泡状态：thinking 等待模型 / working 执行工具 / success|error 收尾闪现 */
const bubbleState = ref<AvatarState>("thinking");
const bubbleVisible = ref(false);
let bubbleHideTimer = 0;

/** 收尾反馈：success/error 表情闪现片刻后隐藏气泡 */
function finishBubble(state: AvatarState, holdMs: number) {
  window.clearTimeout(bubbleHideTimer);
  bubbleState.value = state;
  bubbleHideTimer = window.setTimeout(() => {
    bubbleVisible.value = false;
  }, holdMs);
}

onBeforeUnmount(() => window.clearTimeout(bubbleHideTimer));

/** 广播给桌面宠物（跨窗口）：气泡状态变化时同步表情，隐藏时复位 */
watch([bubbleState, bubbleVisible], () => {
  void emit("pet-state", {
    state: bubbleVisible.value ? bubbleState.value : "idle",
  });
});

/** 点击快捷提示：填入输入框；已有附件时直接发送 */
function useQuickPrompt(key: string) {
  input.value = t(`aiAssistant.quick.${key}`);
  if (attached.value.length) {
    void send();
  } else {
    nextTick(() => inputEl.value?.focus());
  }
}

function scrollToBottom() {
  nextTick(() => {
    const el = listEl.value;
    if (el) el.scrollTop = el.scrollHeight;
  });
}

/** 当前本地时间描述（注入系统提示词，模型自身不知道今天几号） */
function nowText(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  const weekdays = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} (${weekdays[d.getDay()]}) ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** 系统提示词：身份（兔小胖）+ 当前时间 + 能力边界 + 工具规则（附件路径已内联到对应用户消息中，随历史保留） */
function buildSystemPrompt(): string {
  return `You are 兔小胖 (Tù Xiǎopàng), the built-in AI assistant of DocMorph, a desktop document conversion & PDF toolkit app. You operate local files by calling the provided tools (e.g. convert_document, pdf_merge, pdf_compress, pdf_watermark, translate_document).

Current local date and time: ${nowText()}.
- This is the ONLY valid time reference. Use it for any date/time question (e.g. "今天星期几"); never guess or fabricate dates from your training data.

Capabilities and boundaries (be honest, never fabricate):
${searchAvailable()
  ? "- You CAN search the web via the web_search tool. For any real-time or potentially outdated information (news, weather, prices, latest versions, recent events), ALWAYS call web_search first and answer from its results with source links, instead of answering from memory."
  : "- The provided tools are your COMPLETE capability set for touching the outside world. You have NO internet access: you cannot check real-time information such as news, weather, stock prices, exchange rates or sports results. When asked, say plainly that you cannot access the internet; never invent such data."}
- Your knowledge has a training cutoff. For facts that may have changed recently (latest versions, recent events, current rankings), state that your information may be outdated.
- If you are unsure about any fact or answer, say so directly. A short honest "I don't know" is always better than a made-up answer.
- You know this app well and answer usage questions from its actual features: document conversion (PDF / Word / Excel / PPT / e-book formats), PDF processing (merge, split, compress, watermark, encrypt, page numbers, extract pages/images), file tools (rename, image compress/convert), AI document Q&A, translation, batch rename, and folder auto-conversion watching.

Rules:
1. Reply in the user's language (current UI language: ${t("aiAssistant.langName")}). Be concise.
1a. Your name is 兔小胖. When asked who you are, introduce yourself as 兔小胖, DocMorph's assistant — friendly and a little playful, but always focused on getting the user's document work done.
2. Fulfill tasks by calling tools. NEVER invent file paths: only use absolute paths from the conversation (attached files or user-provided ones).
3. Output files are generated automatically in the input file's folder; briefly tell the user where each output file was saved.
4. If required parameters are missing (e.g. watermark text, target format), ask the user instead of guessing.
5. If a tool fails, explain the error and suggest a fix (e.g. install or switch to LibreOffice for document conversion).
6. You may call multiple tools in one turn (e.g. convert then compress).
7. ALWAYS invoke tools through the native function-calling mechanism (tool_calls). NEVER print tool arguments as a JSON text block in your reply.
8. The conversation history is continuous: earlier messages (including tool results) remain visible to you. Refer back to them when the user follows up.`;
}

/** 程序化确认弹窗（危险工具执行前；App.vue 已挂 NDialogProvider） */
function confirmAction(desc: string): Promise<boolean> {
  return new Promise((resolve) => {
    dialog.warning({
      title: t("aiAssistant.confirmTitle"),
      content: desc,
      positiveText: t("aiAssistant.confirmOk"),
      negativeText: t("aiAssistant.confirmCancel"),
      onPositiveClick: () => resolve(true),
      onNegativeClick: () => resolve(false),
      onClose: () => resolve(false),
      onMaskClick: () => resolve(false),
    });
  });
}

/** 执行工具调用；危险操作由工具内部通过 ctx.confirm 请求用户确认（如 batch_rename） */
async function executeToolWithConfirm(call: ToolCall): Promise<ToolResult> {
  const tool = findTool(call.name);
  if (!tool) return { ok: false, message: `Unknown tool: ${call.name}` };
  const ctx: ToolContext = { confirm: confirmAction };
  return executeTool(call, ctx);
}

/** 小模型偶尔不走原生 tool_calls、而是把工具参数当 JSON 文本输出；此处做兼容解析：
 *  提取 ```json 代码块，按参数键集合匹配注册表中的工具（required 全覆盖 + 键均为已知参数） */
function tryExtractTextToolCall(content: string): ToolCall | null {
  const m = content.match(/```(?:json)?\s*([\s\S]*?)```/);
  const candidate = (m ? m[1] : "").trim();
  if (!candidate) return null;
  let args: unknown;
  try {
    args = JSON.parse(candidate);
  } catch {
    return null;
  }
  if (typeof args !== "object" || args === null || Array.isArray(args)) return null;
  const keys = Object.keys(args as object);
  let best: { name: string; score: number } | null = null;
  for (const tool of AI_TOOLS) {
    const schema = tool.parameters as { properties?: Record<string, unknown>; required?: string[] };
    const props = Object.keys(schema.properties ?? {});
    const req = schema.required ?? [];
    if (!req.every((k) => keys.includes(k))) continue;
    if (!keys.every((k) => props.includes(k))) continue;
    if (!best || keys.length > best.score) best = { name: tool.name, score: keys.length };
  }
  if (!best) return null;
  return { id: `text_${Date.now()}`, name: best.name, arguments: candidate };
}

/** 跨轮对话历史（首个元素恒为 system，每次发送刷新；清空对话时重置） */
let chatHistory: ChatMessage[] = [];
/** 历史消息条数上限（不含 system），超出后从最早的完整对话轮裁剪，控制 token 成本 */
const MAX_HISTORY_MESSAGES = 40;

/** 裁剪历史：保留 system + 最近 MAX_HISTORY_MESSAGES 条，裁剪点必须是 user 消息边界
 *  （避免把 assistant(tool_calls) 与对应的 tool 结果切散，破坏 OpenAI 消息序列合法性） */
function trimHistory() {
  while (chatHistory.length > MAX_HISTORY_MESSAGES + 1) {
    const idx = chatHistory.findIndex((m, i) => i >= 1 && m.role === "user");
    if (idx <= 1) break; // 只剩 system + 一条 user，无从裁剪
    chatHistory.splice(1, idx - 1); // 丢弃 system 之后到最早 user 之前的整轮
  }
}

/** 发送指令：LLM 工具调用循环（调用 → 执行 → 结果回填 → 继续） */
async function send() {
  const text = input.value.trim();
  if (!text || busy.value) return;
  if (!cloudReady.value) {
    message.warning(t("aiAssistant.needCloud"));
    return;
  }
  msgs.value.push({
    id: ++seq,
    role: "user",
    content: text,
    // 附件快照到本条消息（气泡下方展示）；随后从输入卡移除，实现"文件跟着消息走"
    files: attached.value.length ? [...attached.value] : undefined,
  });
  input.value = "";
  busy.value = true;
  window.clearTimeout(bubbleHideTimer);
  bubbleState.value = "thinking";
  bubbleVisible.value = true;
  scrollToBottom();

  // 系统提示词每次发送刷新置顶（时间 / 搜索能力等即时生效），历史对话在其后延续
  const systemMsg: ChatMessage = { role: "system", content: buildSystemPrompt() };
  if (chatHistory.length === 0) chatHistory = [systemMsg];
  else chatHistory[0] = systemMsg;

  // 附件路径内联到用户消息（随历史持久保留；此前的做法放 system 会被下次刷新冲掉）
  const filesNote = attached.value.length
    ? `\n\n[Attached files (absolute paths, use them directly in tool calls):\n${attached.value.map((f) => `- ${f}`).join("\n")}]`
    : "";
  chatHistory.push({ role: "user", content: text + filesNote });
  attached.value = [];

  const history = chatHistory;
  try {
    let finished = false;
    for (let turn = 0; turn < MAX_TOOL_TURNS && !finished; turn++) {
      bubbleState.value = "thinking";
      const reply = await chatWithTools(history, getToolDefinitions() as ToolDefinition[]);
      let calls = reply.tool_calls;
      if (!calls.length) {
        // 兼容回退：模型把工具参数当文本输出时，解析后按工具调用执行
        const extracted = reply.content ? tryExtractTextToolCall(reply.content) : null;
        if (!extracted) {
          const replyText = reply.content || t("aiAssistant.emptyReply");
          msgs.value.push({ id: ++seq, role: "assistant", content: replyText });
          history.push({ role: "assistant", content: replyText });
          finished = true;
          break;
        }
        calls = [extracted];
      }
      // assistant 消息携带工具调用，回传给模型
      history.push({
        role: "assistant",
        content: null,
        tool_calls: calls.map((tc) => ({
          id: tc.id, type: "function" as const, function: { name: tc.name, arguments: tc.arguments },
        })),
      });
      for (const call of calls) {
        const uiId = ++seq;
        bubbleState.value = "working";
        msgs.value.push({ id: uiId, role: "tool", content: "", toolName: call.name, running: true });
        scrollToBottom();
        const res = await executeToolWithConfirm(call);
        const m = msgs.value.find((x) => x.id === uiId);
        if (m) {
          m.content = res.message;
          m.ok = res.ok;
          m.running = false;
          m.outputs = res.outputs;
        }
        history.push({ role: "tool", tool_call_id: call.id, content: res.message });
        scrollToBottom();
      }
    }
    if (!finished) {
      const fallback = t("aiAssistant.maxTurns");
      msgs.value.push({ id: ++seq, role: "assistant", content: fallback });
      history.push({ role: "assistant", content: fallback });
    }
    finishBubble("success", 1000);
  } catch (e: any) {
    const failText = t("aiAssistant.fail", { err: String(e) });
    msgs.value.push({ id: ++seq, role: "assistant", content: failText });
    // 错误回复也入历史：保持 user/assistant 交替，避免下轮出现连续 user 消息
    history.push({ role: "assistant", content: "(上一条请求失败，可重试)" });
    finishBubble("error", 1600);
  } finally {
    trimHistory();
    busy.value = false;
  }
}

function clearChat() {
  msgs.value = [];
  input.value = "";
  chatHistory = []; // 对话历史一并清空，下轮从全新上下文开始
}

/** 选择附件文件 */
async function pickFiles() {
  const sel = await open({ multiple: true });
  if (sel) {
    attached.value.push(...sel.map(String));
  }
}

function removeFile(i: number) {
  attached.value.splice(i, 1);
}

/** 文件类型图标 + 颜色（chip 一眼可辨文件类型） */
function fileMeta(path: string): { icon: Component; color: string } {
  const ext = (path.split(".").pop() || "").toLowerCase();
  if (ext === "pdf") return { icon: DocumentTextOutline, color: "#e5484d" };
  if (["doc", "docx", "odt", "rtf", "txt", "md"].includes(ext)) return { icon: DocumentTextOutline, color: "#3b82f6" };
  if (["xls", "xlsx", "ods", "csv"].includes(ext)) return { icon: GridOutline, color: "#22a06b" };
  if (["ppt", "pptx", "odp"].includes(ext)) return { icon: EaselOutline, color: "#f76b15" };
  if (["png", "jpg", "jpeg", "bmp", "gif", "webp", "svg"].includes(ext)) return { icon: ImageOutline, color: "#8e4ec6" };
  if (["html", "htm"].includes(ext)) return { icon: GlobeOutline, color: "#0ea5e9" };
  if (["zip", "rar", "7z", "tar", "gz"].includes(ext)) return { icon: ArchiveOutline, color: "#a17c0b" };
  if (["mp3", "wav", "flac", "aac", "ogg"].includes(ext)) return { icon: MusicalNotesOutline, color: "#ec4899" };
  if (["mp4", "mov", "avi", "mkv", "webm"].includes(ext)) return { icon: FilmOutline, color: "#6366f1" };
  return { icon: DocumentOutline, color: "var(--text-muted)" };
}

function onDrop(e: DragEvent) {
  dragOver.value = false;
  const files = e.dataTransfer?.files;
  if (files) {
    attached.value.push(...Array.from(files).map((f) => (f as any).path).filter(Boolean));
  }
}

/** 拖拽离开输入卡：取消高亮（dragleave 会冒泡自子元素，进出计数归零才算离开） */
let dragDepth = 0;
function onDragEnter() {
  dragDepth++;
  dragOver.value = true;
}
function onDragLeave() {
  dragDepth = Math.max(0, dragDepth - 1);
  if (dragDepth === 0) dragOver.value = false;
}
const dragOver = ref(false);

/** 供 Home.vue 拖拽分发：把拖入的文件加入附件 */
defineExpose({
  handleDrop: (paths: string[]) => {
    attached.value.push(...paths);
  },
});
</script>

<style scoped>
.assistant-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  gap: 12px;
}
.cloud-warn {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  font-size: 12px;
  color: var(--orange);
  background: var(--orange-soft);
  border-radius: 10px;
  padding: 10px 14px;
}
/* 输入卡片：附件与输入框一体化 */
.input-card {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  border: 1px solid var(--border-strong);
  border-radius: 14px;
  background: var(--bg-input);
  padding: 12px;
  transition: border-color 0.18s, box-shadow 0.18s;
}
.input-card:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}
/* 拖文件悬停高亮：提示可放置 */
.input-card.drag-over {
  border-color: var(--accent);
  background: var(--accent-soft);
  box-shadow: 0 0 0 3px var(--accent-soft);
}
.attach-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.attach-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--text-sub);
  background: var(--bg-tag);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 6px 8px;
  max-width: 260px;
}
.chip-icon {
  display: inline-flex;
  flex-shrink: 0;
}
.chip-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.chip-name {
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 180px;
}
.chip-ext {
  font-size: 10px;
  color: var(--text-muted);
}
.chip-close {
  border: none;
  background: none;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
}
.chip-close:hover {
  color: var(--red);
}
/* 消息列表 */
.msg-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px;
}
/* 细滚动条跟随主题 */
.msg-list::-webkit-scrollbar {
  width: 5px;
}
.msg-list::-webkit-scrollbar-track {
  background: transparent;
}
.msg-list::-webkit-scrollbar-thumb {
  background: var(--border-strong);
  border-radius: 3px;
}
.msg-list::-webkit-scrollbar-thumb:hover {
  background: var(--text-faint);
}
.msg-empty {
  margin: auto;
  text-align: center;
  max-width: 600px;
  width: 100%;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.8;
}
/* 快捷提示：AI 头像（双层 Lottie：脸 + 瞳孔视线跟随）+ 标题 + 提示卡片网格 */
/* 助手状态气泡：迷你头像 + 三点跳动（消息流末尾）；卡片风与助手气泡一致 */
.typing-row {
  display: flex;
  align-items: center;
  gap: 10px;
  align-self: flex-start;
  padding: 8px 12px;
  margin-top: 8px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  box-shadow: 0 1px 2px var(--shadow);
  border-radius: 14px;
  animation: msg-in 0.2s ease-out;
}
.typing-dots {
  display: flex;
  gap: 4px;
}
.typing-dots span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
  animation: typing-bounce 1.2s ease-in-out infinite;
}
.typing-dots span:nth-child(2) {
  animation-delay: 0.15s;
}
.typing-dots span:nth-child(3) {
  animation-delay: 0.3s;
}
@keyframes typing-bounce {
  0%, 100% { transform: translateY(0); opacity: 0.5; }
  50% { transform: translateY(-4px); opacity: 1; }
}
@media (prefers-reduced-motion: reduce) {
  .typing-dots span,
  .bubble,
  .typing-row {
    animation: none;
  }
}

.quick-title {
  margin: 16px 0 4px;
  font-size: 17px;
  font-weight: 600;
  color: var(--text-main);
}
.quick-sub {
  margin: 0 0 16px;
  font-size: 12px;
  color: var(--text-muted);
}
.quick-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
  width: 100%;
  max-width: 560px;
  margin: 0 auto;
}
.quick-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  background: var(--bg-tag);
  border: 1px solid var(--border);
  border-radius: 12px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-body);
  text-align: left;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, transform 0.12s;
}
.quick-card:hover {
  border-color: var(--accent);
  background: var(--bg-input);
  transform: translateY(-1px);
}
.quick-card:active {
  transform: translateY(0) scale(0.98);
}
.quick-emoji {
  font-size: 20px;
  flex-shrink: 0;
}
.quick-text {
  flex: 1;
  min-width: 0;
}
.msg-row.user {
  align-self: flex-end;
}
/* 助手侧行：迷你头像 + 内容横向排列（文本气泡 / 工具卡片） */
.msg-row.assistant,
.msg-row.tool {
  align-self: flex-start;
  display: flex;
  align-items: flex-start;
  gap: 8px;
  max-width: 100%;
}
.row-avatar {
  flex-shrink: 0;
  /* 与气泡首行文字光学对齐（气泡上内边距 10px + 标签行高） */
  margin-top: 2px;
}
.msg-row.tool .row-avatar {
  margin-top: 6px; /* 工具卡片有边框，头像略下移对齐标题行 */
}
/* 头像出现后，助手侧内容不再受 82% 限制挤压，交由行容器控制 */
.msg-row.assistant .bubble,
.msg-row.tool .tool-card {
  max-width: calc(100% - 44px);
}
.bubble {
  display: flex;
  flex-direction: column;
  gap: 3px;
  max-width: 82%;
  border-radius: 12px;
  padding: 10px 14px;
  font-size: 13px;
  line-height: 1.7;
  word-break: break-word;
  white-space: pre-wrap;
  animation: msg-in 0.2s ease-out;
}
/* 用户气泡跟随 CTA 单色系（浅色主题黑底 / 深色主题白底），与应用主按钮同语言 */
.bubble.user {
  background: var(--cta-bg);
  color: var(--cta-text);
  border-bottom-right-radius: 4px;
}
/* 助手气泡走面板卡片风：白底细边框柔和阴影，与各功能面板一致 */
.bubble.assistant {
  background: var(--bg-panel);
  color: var(--text-body);
  border: 1px solid var(--border);
  box-shadow: 0 1px 2px var(--shadow);
  border-bottom-left-radius: 4px;
}
@keyframes msg-in {
  from {
    opacity: 0;
    transform: translateY(6px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
.who {
  font-size: 11px;
  opacity: 0.72;
}
/* 用户消息关联的附件（气泡内下方，点击打开文件） */
.msg-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 4px;
}
.msg-file-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  /* 中性半透明：浅色主题（黑气泡）/ 深色主题（白气泡）都成立 */
  border: 1px solid rgba(127, 127, 127, 0.35);
  background: rgba(127, 127, 127, 0.16);
  border-radius: 8px;
  padding: 4px 8px;
  cursor: pointer;
  transition: background 0.15s, transform 0.12s;
}
.msg-file-chip:hover {
  background: rgba(127, 127, 127, 0.28);
}
.msg-file-chip:active {
  transform: scale(0.97);
}
.msg-file-name {
  font-size: 12px;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* 附件 chip 图标颜色跟随气泡文字色，两种主题自适应 */
.bubble.user .chip-icon {
  color: var(--cta-text) !important;
}
/* 工具执行卡片 */
.tool-card {
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  padding: 10px 14px;
  background: var(--bg-page);
  max-width: 82%;
}
.tool-card.ok {
  border-color: var(--green-soft);
}
.tool-card.fail {
  border-color: var(--red-soft);
}
.tool-head {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-sub);
}
.tool-head .spin {
  animation: rotate 1s linear infinite;
}
.tool-card.ok .tool-head {
  color: var(--green);
}
.tool-card.fail .tool-head {
  color: var(--red);
}
.tool-name {
  font-weight: 600;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
.tool-state {
  margin-left: auto;
  font-size: 11px;
}
.tool-body {
  margin-top: 8px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-body);
  white-space: pre-wrap;
  word-break: break-word;
}
/* 工具成功后的打开入口（与各功能面板 ResultBar 交互一致） */
.tool-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
.tool-open-btn {
  border: 1px solid var(--border-strong);
  background: var(--bg-tag);
  color: var(--accent);
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, transform 0.12s;
}
.tool-open-btn:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.tool-open-btn:active {
  transform: scale(0.96);
}
/* 输入卡片内部 */
.chat-input {
  width: 100%;
  border: none;
  background: transparent;
  resize: none;
  outline: none;
  font: inherit;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-main);
  min-height: 40px;
  max-height: 140px;
  box-sizing: border-box;
}
.chat-input::placeholder {
  color: var(--text-faint);
}
.card-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.actions-spacer {
  flex: 1;
}
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: 1px solid var(--border-strong);
  border-radius: 9px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s, border-color 0.15s, background 0.15s, transform 0.12s;
}
/* 网页搜索开关激活态：地球图标高亮 */
.icon-btn.active {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}
.icon-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--bg-tag);
}
.icon-btn:active:not(:disabled) {
  transform: scale(0.94);
}
.icon-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.send-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 10px;
  background: var(--cta-bg);
  color: var(--cta-text);
  cursor: pointer;
  flex-shrink: 0;
  transition: opacity 0.15s, transform 0.15s;
}
.send-btn:hover:not(:disabled) {
  opacity: 0.85;
  transform: translateY(-1px);
}
.send-btn:active:not(:disabled) {
  transform: translateY(0) scale(0.94);
}
.send-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
}
.send-btn .spin {
  animation: rotate 1s linear infinite;
}
@keyframes rotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
