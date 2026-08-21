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
        <div ref="avatarEl" class="quick-avatar" aria-hidden="true">
          <div class="avatar-halo"></div>
          <div class="avatar-bob">
            <div ref="baseAnimEl" class="avatar-anim"></div>
            <div class="avatar-eyes" :style="eyesStyle">
              <div ref="eyesAnimEl" class="avatar-anim"></div>
            </div>
          </div>
        </div>
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
          <span class="who">{{ m.role === "user" ? t("aiAssistant.you") : t("aiAssistant.assistant") }}</span>
          <span class="text">{{ m.content }}</span>
        </div>
      </div>
    </div>

    <!-- 输入卡片：附件与输入框一体化（点击 + 或拖拽文件到卡片添加附件） -->
    <div class="input-card" @dragover.prevent @drop.prevent="onDrop">
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, type Component } from "vue";
import { NIcon, useDialog, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import {
  AddOutline, ArchiveOutline, CheckmarkCircleOutline, CloseCircleOutline, CloudOfflineOutline,
  DocumentOutline, DocumentTextOutline, EaselOutline, FilmOutline, GlobeOutline, GridOutline, ImageOutline,
  MusicalNotesOutline, SendOutline, SyncOutline, TrashOutline,
} from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import lottie from "lottie-web";
import type { AnimationItem } from "lottie-web";
import avatarAnimData from "../assets/ai-avatar.json";
import avatarEyesData from "../assets/ai-avatar-eyes.json";
import { useSettingsStore } from "../stores/settings";
import { chatWithTools } from "../ai";
import type { ChatMessage, ToolCall, ToolDefinition } from "../ai";
import { AI_TOOLS, executeTool, findTool, TOOL_DEFINITIONS } from "../ai/tools";
import type { ToolContext, ToolResult } from "../ai/tools";
import { openPath } from "../api";
import { dirOf } from "../utils/file";

const { t } = useI18n();
const message = useMessage();
const dialog = useDialog();
const settings = useSettingsStore();

/** 云端配置就绪（助手依赖云端模型，本地模型不支持工具调用） */
const cloudReady = computed(() => !!(settings.ai.cloud.baseUrl && settings.ai.cloud.apiKey));

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
  toolName?: string;
  ok?: boolean;
  running?: boolean;
  /** 工具成功时的输出文件路径（卡片上提供打开入口） */
  outputs?: string[];
}
const msgs = ref<UiMsg[]>([]);
let seq = 0;

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

/* ---------- 空状态 Lottie 头像：底层动画 + 瞳孔层（视线跟随鼠标） ---------- */
const avatarEl = ref<HTMLElement | null>(null);
const baseAnimEl = ref<HTMLElement | null>(null);
const eyesAnimEl = ref<HTMLElement | null>(null);
let baseAnim: AnimationItem | null = null;
let eyesAnim: AnimationItem | null = null;
let eyesSyncTimer = 0;

/** 瞳孔偏移（屏幕像素，朝鼠标方向） */
const eyesShift = ref({ x: 0, y: 0 });
const eyesStyle = computed(() => ({
  transform: `translate(${eyesShift.value.x.toFixed(1)}px, ${eyesShift.value.y.toFixed(1)}px)`,
}));

/** 初始化双层动画（容器存在且未初始化时） */
function mountAvatarAnim() {
  if (baseAnim || !baseAnimEl.value || !eyesAnimEl.value) return;
  baseAnim = lottie.loadAnimation({
    container: baseAnimEl.value,
    renderer: "svg",
    loop: true,
    autoplay: true,
    animationData: avatarAnimData,
  });
  eyesAnim = lottie.loadAnimation({
    container: eyesAnimEl.value,
    renderer: "svg",
    loop: true,
    autoplay: true,
    animationData: avatarEyesData,
  });
  // 双实例独立计时，定期以底层帧校准瞳孔层，保证眨眼同步
  eyesSyncTimer = window.setInterval(() => {
    if (baseAnim && eyesAnim) eyesAnim.goToAndPlay(baseAnim.currentFrame, true);
  }, 1000);
  window.addEventListener("mousemove", onAvatarPointerMove, { passive: true });
  document.addEventListener("mouseleave", resetEyesShift);
}

/** 销毁动画与监听，释放资源 */
function unmountAvatarAnim() {
  window.clearInterval(eyesSyncTimer);
  window.removeEventListener("mousemove", onAvatarPointerMove);
  document.removeEventListener("mouseleave", resetEyesShift);
  baseAnim?.destroy();
  eyesAnim?.destroy();
  baseAnim = null;
  eyesAnim = null;
  eyesShift.value = { x: 0, y: 0 };
}

/** 视线跟随：瞳孔朝鼠标方向偏移，距离越远越明显（封顶后保持看向该方向） */
function onAvatarPointerMove(e: MouseEvent) {
  const host = avatarEl.value;
  if (!host) return;
  const r = host.getBoundingClientRect();
  const dx = e.clientX - (r.left + r.width / 2);
  const dy = e.clientY - (r.top + r.height * 0.47); // 眼睛约在头部 47% 高度处
  const dist = Math.hypot(dx, dy) || 1;
  const unit = r.width / 240; // 画布单位 → 屏幕像素
  const reach = Math.min(1, dist / 150); // 近距离衰减，避免在脸上打转时抖动
  eyesShift.value = {
    x: (dx / dist) * 6.5 * unit * reach,
    y: (dy / dist) * 7 * unit * reach,
  };
}

/** 鼠标离开窗口：视线回中 */
function resetEyesShift() {
  eyesShift.value = { x: 0, y: 0 };
}

onMounted(mountAvatarAnim);
// 空状态由 v-if 控制：发送首条消息时卸载、清空对话时重建，动画跟随容器挂载/销毁
watch([baseAnimEl, eyesAnimEl], ([b, e]) => (b && e ? mountAvatarAnim() : unmountAvatarAnim()));
onBeforeUnmount(unmountAvatarAnim);

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

/** 系统提示词：工具使用规则 + 附件文件路径（发给云端模型） */
function buildSystemPrompt(): string {
  const filesBlock = attached.value.length
    ? `\n\nAttached files (absolute paths, use them directly in tool calls):\n${attached.value.map((f) => `- ${f}`).join("\n")}`
    : "";
  return `You are the built-in AI assistant of DocMorph, a desktop document conversion & PDF toolkit app. You operate local files by calling the provided tools (e.g. convert_document, pdf_merge, pdf_compress, pdf_watermark).

Rules:
1. Reply in the user's language (current UI language: ${t("aiAssistant.langName")}). Be concise.
2. Fulfill tasks by calling tools. NEVER invent file paths: only use absolute paths from the conversation (attached files or user-provided ones).
3. Output files are generated automatically in the input file's folder; briefly tell the user where each output file was saved.
4. If required parameters are missing (e.g. watermark text, target format), ask the user instead of guessing.
5. If a tool fails, explain the error and suggest a fix (e.g. install or switch to LibreOffice for document conversion).
6. You may call multiple tools in one turn (e.g. convert then compress).
7. ALWAYS invoke tools through the native function-calling mechanism (tool_calls). NEVER print tool arguments as a JSON text block in your reply.${filesBlock}`;
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

/** 执行工具调用；危险工具先请求用户确认 */
async function executeToolWithConfirm(call: ToolCall): Promise<ToolResult> {
  const tool = findTool(call.name);
  if (!tool) return { ok: false, message: `Unknown tool: ${call.name}` };
  const ctx: ToolContext = { confirm: confirmAction };
  if (tool.dangerous) {
    const ok = await confirmAction(t("aiAssistant.confirmBody", { tool: call.name, args: call.arguments }));
    if (!ok) return { ok: false, message: t("aiAssistant.cancelled") };
  }
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

/** 发送指令：LLM 工具调用循环（调用 → 执行 → 结果回填 → 继续） */
async function send() {
  const text = input.value.trim();
  if (!text || busy.value) return;
  if (!cloudReady.value) {
    message.warning(t("aiAssistant.needCloud"));
    return;
  }
  msgs.value.push({ id: ++seq, role: "user", content: text });
  input.value = "";
  busy.value = true;
  scrollToBottom();

  const history: ChatMessage[] = [
    { role: "system", content: buildSystemPrompt() },
    { role: "user", content: text },
  ];
  try {
    let finished = false;
    for (let turn = 0; turn < MAX_TOOL_TURNS && !finished; turn++) {
      const reply = await chatWithTools(history, TOOL_DEFINITIONS as ToolDefinition[]);
      let calls = reply.tool_calls;
      if (!calls.length) {
        // 兼容回退：模型把工具参数当文本输出时，解析后按工具调用执行
        const extracted = reply.content ? tryExtractTextToolCall(reply.content) : null;
        if (!extracted) {
          msgs.value.push({ id: ++seq, role: "assistant", content: reply.content || t("aiAssistant.emptyReply") });
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
      msgs.value.push({ id: ++seq, role: "assistant", content: t("aiAssistant.maxTurns") });
    }
  } catch (e: any) {
    msgs.value.push({ id: ++seq, role: "assistant", content: t("aiAssistant.fail", { err: String(e) }) });
  } finally {
    busy.value = false;
  }
}

function clearChat() {
  msgs.value = [];
  input.value = "";
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
  const files = e.dataTransfer?.files;
  if (files) {
    attached.value.push(...Array.from(files).map((f) => (f as any).path).filter(Boolean));
  }
}

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
  transition: border-color 0.15s;
}
.input-card:focus-within {
  border-color: var(--accent);
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
  padding: 4px 2px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px;
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
.quick-avatar {
  position: relative;
  width: 132px;
  height: 132px;
  margin: 0 auto;
}
/* 光晕 + 悬浮阴影（不随头部浮动，营造悬浮感） */
.avatar-halo {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.avatar-halo::before {
  content: "";
  position: absolute;
  left: 50%;
  top: 50%;
  width: 72%;
  height: 72%;
  border-radius: 50%;
  background: var(--accent);
  transform: translate(-50%, -50%);
  animation: halo-pulse 4s ease-in-out infinite;
}
.avatar-halo::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 2px;
  width: 47%;
  height: 7%;
  border-radius: 50%;
  background: var(--accent);
  transform: translateX(-50%);
  animation: shadow-pulse 4s ease-in-out infinite;
}
/* 头部整体浮动（脸 + 瞳孔同层，天然同步） */
.avatar-bob {
  position: absolute;
  inset: 0;
  animation: avatar-bob 4s ease-in-out infinite;
}
.avatar-anim {
  position: absolute;
  inset: 0;
}
/* 瞳孔层：JS 驱动 translate，transition 平滑视线移动 */
.avatar-eyes {
  position: absolute;
  inset: 0;
  transition: transform 0.09s ease-out;
  will-change: transform;
}
@keyframes avatar-bob {
  0%, 100% { transform: translateY(3px); }
  50% { transform: translateY(-3px); }
}
@keyframes halo-pulse {
  0%, 100% { opacity: 0.07; transform: translate(-50%, -50%) scale(0.94); }
  50% { opacity: 0.13; transform: translate(-50%, -50%) scale(1.06); }
}
@keyframes shadow-pulse {
  0%, 100% { opacity: 0.14; }
  50% { opacity: 0.07; }
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
.msg-row.assistant {
  align-self: flex-start;
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
}
.bubble.user {
  background: var(--accent);
  color: #fff;
  border-bottom-right-radius: 4px;
}
.bubble.assistant {
  background: var(--bg-tag);
  color: var(--text-body);
  border-bottom-left-radius: 4px;
}
.who {
  font-size: 11px;
  opacity: 0.75;
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
  transition: border-color 0.15s;
}
.tool-open-btn:hover {
  border-color: var(--accent);
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
  transition: color 0.15s, border-color 0.15s;
}
.icon-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
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
  transition: opacity 0.15s;
}
.send-btn:hover:not(:disabled) {
  opacity: 0.85;
}
.send-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.send-btn .spin {
  animation: rotate 1s linear infinite;
}
@keyframes rotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
