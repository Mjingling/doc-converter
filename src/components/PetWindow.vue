<template>
  <div class="pet-root" @contextmenu.prevent="toggleMenu" @click="closeOverlays">
    <!-- 气泡：小贴士 / 任务进度 / 完成与失败反馈 -->
    <Transition name="bubble">
      <div v-if="bubble" class="pet-bubble" :class="`bubble-${bubble.kind}`">
        <template v-if="bubble.kind === 'progress'">
          <span class="bubble-text">{{ t("pet.progress") }}</span>
          <div class="pet-progress">
            <div
              class="pet-progress-bar"
              :class="{ indeterminate: bubble.progress == null }"
              :style="bubble.progress != null ? { width: `${bubble.progress}%` } : undefined"
            ></div>
          </div>
        </template>
        <span v-else class="bubble-text">{{ bubble.text }}</span>
      </div>
    </Transition>

    <!-- 摸头 / 戳一戳：爱心上浮 -->
    <div class="pet-hearts">
      <span v-for="h in hearts" :key="h.id" class="heart" :style="{ left: `${h.x}px`, animationDelay: `${h.delay}ms` }">♥</span>
    </div>

    <!-- 包装层：空闲行为与戳一戳的小跳/摇摆动画（不进头像组件，避免污染状态语义） -->
    <div
      class="pet-anim"
      :class="wrapperAnim"
      @pointerdown="onPointerDown"
      @pointerup="onPointerUp"
      @pointerenter="onHoverEnter"
    >
      <AssistantAvatar
        size="lg"
        :state="displayState"
        :eye-shift="lookShift"
        track="global"
      />
    </div>

    <!-- 右键菜单：快捷功能 + 助手 + 隐藏（原双击面板并入此处，点外部/Esc/点击菜单项即关闭） -->
    <div v-if="menuOpen" class="pet-card" @click.stop>
      <button
        v-for="q in QUICK_ITEMS"
        :key="q.id"
        class="pet-menu-item"
        @click="onQuick(q.id)"
      >
        <span class="menu-emoji">{{ q.icon }}</span>{{ t(`nav.${q.id}`) }}
      </button>
      <div class="pet-menu-divider"></div>
      <button class="pet-menu-item" @click="onMenuAssistant">{{ t("pet.menu.openAssistant") }}</button>
      <button class="pet-menu-item" @click="onMenuHide">{{ t("pet.menu.hide") }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import AssistantAvatar, { type AvatarState } from "./AssistantAvatar.vue";
import { petHide, petOpenMain } from "../api";
import type { PetProgressPayload } from "../utils/petProgress";
import {
  aiStateHoldMs, nextBehaviorDelay, nextTipDelay,
  pickBehavior, pickPokeReaction, resolveDisplayState,
} from "../utils/petBehavior";

const { t, tm } = useI18n();

/* ---------- 显示状态仲裁：任务表情 > AI 状态事件 > 空闲行为（打盹）> idle ---------- */
const aiState = ref<AvatarState | null>(null);
/** AI 状态过期时间戳（ms；null = 持续显示直到下一条事件） */
const aiUntil = ref<number | null>(null);
const dozing = ref(false);
/** 任务完成/失败的临时表情（优先级最高，定时回退） */
const faceOverride = ref<AvatarState | null>(null);
/** 过期检测用的时钟（success/error 短闪需要时间驱动回退） */
const nowTick = ref(Date.now());

const displayState = computed(() =>
  faceOverride.value ?? resolveDisplayState(aiState.value, aiUntil.value, dozing.value, nowTick.value),
);

/** AI 状态是否活跃（未过期）：忙碌守卫用，避免 AI 期间宠物自顾自做动作 */
const aiActive = computed(() => {
  if (!aiState.value || aiState.value === "idle") return false;
  return aiUntil.value === null || nowTick.value < aiUntil.value;
});

let faceTimer = 0;
function setFaceOverride(state: AvatarState, holdMs: number) {
  window.clearTimeout(faceTimer);
  faceOverride.value = state;
  faceTimer = window.setTimeout(() => (faceOverride.value = null), holdMs);
}

/* ---------- AI 状态同步（主窗口 AiAssistantPanel 广播） ---------- */
let unlistenState: UnlistenFn | null = null;

function applyAiState(state: AvatarState) {
  interruptBehavior(); // AI 有动静：打断空闲行为（含打盹），但保持调度链存活
  faceOverride.value = null; // AI 表情接管任务表情
  if (state === "idle") {
    aiState.value = null;
    aiUntil.value = null;
  } else {
    aiState.value = state;
    const hold = aiStateHoldMs(state);
    aiUntil.value = hold === null ? null : Date.now() + hold;
  }
  nowTick.value = Date.now();
}

/* ---------- 气泡：小贴士 / 进度 / 完成反馈 ---------- */
interface Bubble {
  kind: "tip" | "progress" | "success" | "error";
  text: string;
  /** 进度条数值 0~100；缺省为不确定态 */
  progress?: number;
}
const bubble = ref<Bubble | null>(null);
let bubbleTimer = 0;

/** 显示气泡；holdMs 为 null 时持续显示（进度条场景） */
function showBubble(b: Bubble, holdMs: number | null) {
  window.clearTimeout(bubbleTimer);
  bubble.value = b;
  if (holdMs !== null) bubbleTimer = window.setTimeout(() => (bubble.value = null), holdMs);
}

/** 小贴士文案（i18n 数组） */
const tips = computed(() => (tm("pet.tips") as unknown as string[]) ?? []);

function showRandomTip() {
  const pool = tips.value;
  if (!pool.length) return;
  showBubble({ kind: "tip", text: pool[Math.floor(Math.random() * pool.length)] }, 4500);
}

/* ---------- 任务进度反馈（主窗口 usePanelTask / history 广播） ---------- */
let unlistenProgress: UnlistenFn | null = null;

function applyProgress(p: PetProgressPayload) {
  interruptBehavior();
  faceOverride.value = null;
  switch (p.phase) {
    case "start":
      showBubble({ kind: "progress", text: "", progress: p.progress }, null);
      break;
    case "tick":
      if (bubble.value?.kind === "progress") bubble.value.progress = p.progress;
      break;
    case "done":
      showBubble({ kind: "success", text: t("pet.doneMsg") }, 2500);
      setFaceOverride("success", 2500);
      break;
    case "error":
      showBubble({ kind: "error", text: t("pet.errorMsg") }, 3200);
      setFaceOverride("error", 3200);
      break;
  }
}

/* ---------- 空闲行为调度器：张望 / 打盹 / 小跳 / 摇摆 ---------- */
const lookShift = ref<{ x: number; y: number } | null>(null);
const wrapperAnim = ref<"" | "pet-hop" | "pet-wiggle">("");
let nextTimer = 0;
let behaviorTimers: number[] = [];

function clearBehaviorTimers() {
  for (const t of behaviorTimers) window.clearTimeout(t);
  behaviorTimers = [];
}

function cancelBehavior() {
  clearBehaviorTimers();
  dozing.value = false;
  lookShift.value = null;
  wrapperAnim.value = "";
}

function scheduleNext() {
  window.clearTimeout(nextTimer);
  nextTimer = window.setTimeout(runBehavior, nextBehaviorDelay(Math.random()));
}

/**
 * 外部打断（交互 / AI / 任务事件）：清掉进行中的行为，但重新排下一次空闲行为。
 * 只用 cancelBehavior 会把续链定时器一并清掉，导致宠物从此永远停在 idle（只剩眨眼）。
 */
function interruptBehavior() {
  cancelBehavior();
  scheduleNext();
}

function runBehavior() {
  cancelBehavior();
  // 忙碌期（AI 活跃 / 任务表情 / 进度条）：不自顾自做动作，稍后再试
  if (aiActive.value || faceOverride.value || bubble.value?.kind === "progress") {
    scheduleNext();
    return;
  }
  const b = pickBehavior(Math.random());
  const later = (ms: number, fn: () => void) => behaviorTimers.push(window.setTimeout(fn, ms));
  switch (b.kind) {
    case "lookAround":
      // 左 → 右 → 回中扫视
      lookShift.value = { x: -8, y: 1 };
      later(850, () => (lookShift.value = { x: 8, y: 1 }));
      later(1700, () => (lookShift.value = { x: -5, y: 1 }));
      later(b.duration, () => (lookShift.value = null));
      break;
    case "doze":
      dozing.value = true; // 鼠标移入 / 点击 / AI 事件立即唤醒（见 onHoverEnter / applyAiState）
      break;
    case "hop":
      wrapperAnim.value = "pet-hop";
      later(b.duration, () => (wrapperAnim.value = ""));
      break;
    case "wiggle":
      wrapperAnim.value = "pet-wiggle";
      later(b.duration, () => (wrapperAnim.value = ""));
      break;
  }
  later(b.duration, scheduleNext);
}

/* ---------- 随机小贴士调度：空闲且无气泡时偶尔冒一句 ---------- */
let tipTimer = 0;

function scheduleTip() {
  window.clearTimeout(tipTimer);
  tipTimer = window.setTimeout(() => {
    if (!bubble.value && !aiState.value && !faceOverride.value) showRandomTip();
    scheduleTip();
  }, nextTipDelay(Math.random()));
}

/* ---------- 摸头爱心 ---------- */
interface Heart { id: number; x: number; delay: number }
const hearts = ref<Heart[]>([]);
let heartSeq = 0;
let lastHeartAt = 0;

function spawnHearts(count: number, throttleMs = 0) {
  const now = Date.now();
  if (throttleMs && now - lastHeartAt < throttleMs) return;
  lastHeartAt = now;
  const batch: Heart[] = Array.from({ length: count }, () => ({
    id: heartSeq++,
    x: 34 + Math.random() * 64,
    delay: Math.random() * 350,
  }));
  hearts.value.push(...batch);
  window.setTimeout(() => {
    const ids = new Set(batch.map((h) => h.id));
    hearts.value = hearts.value.filter((h) => !ids.has(h.id));
  }, 1700);
}

/* ---------- 交互：单击戳一戳 / 右键菜单（含快捷功能） / 拖动移动 ---------- */
const menuOpen = ref(false);

/** 右键菜单的快捷功能入口（nav id + 表情符号） */
const QUICK_ITEMS = [
  { id: "compress", icon: "🗜️" },
  { id: "merge", icon: "🧩" },
  { id: "images2pdf", icon: "🖼️" },
  { id: "convert", icon: "🔄" },
] as const;

function closeOverlays() {
  menuOpen.value = false;
}

function toggleMenu() {
  menuOpen.value = !menuOpen.value;
  if (dozing.value) interruptBehavior(); // 右键也算唤醒
}

function onMenuAssistant() {
  closeOverlays();
  void petOpenMain(); // 无面板参数 → 切到 AI 助手
}

function onMenuHide() {
  void petHide();
}

function onQuick(id: string) {
  closeOverlays();
  void petOpenMain(id); // 唤起主窗口并切到对应功能面板
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") closeOverlays();
}

/* ---------- 戳一戳：单击随机反应（小跳 / 摇摆 / 爱心） ---------- */
let downPos: { x: number; y: number } | null = null;
let dragged = false;

function onPointerDown(e: PointerEvent) {
  if (e.button !== 0) return; // 仅主键参与单击/拖动：右键交给 contextmenu 菜单
  downPos = { x: e.screenX, y: e.screenY };
  dragged = false;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  // 按下即唤醒打盹
  if (dozing.value) interruptBehavior();
}

function onPointerMove(e: PointerEvent) {
  if (!downPos || dragged) return;
  const dx = e.screenX - downPos.x;
  const dy = e.screenY - downPos.y;
  if (Math.hypot(dx, dy) > 6) {
    dragged = true;
    import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().startDragging());
  }
}

function onPointerUp(e: PointerEvent) {
  if (e.button !== 0) return; // 右键松开不算单击：菜单刚由 contextmenu 打开，不能立即被关闭
  if (downPos && !dragged) {
    // 单击：戳一戳反应（双击时也会触发两次，反应叠加无副作用）
    closeOverlays();
    poke();
  }
  downPos = null;
}

function poke() {
  interruptBehavior();
  faceOverride.value = null;
  const r = pickPokeReaction(Math.random());
  switch (r) {
    case "hop":
      wrapperAnim.value = "pet-hop";
      window.setTimeout(() => (wrapperAnim.value = ""), 900);
      break;
    case "wiggle":
      wrapperAnim.value = "pet-wiggle";
      window.setTimeout(() => (wrapperAnim.value = ""), 1200);
      break;
    case "hearts":
      spawnHearts(4);
      break;
  }
}

function onHoverEnter() {
  if (dozing.value) interruptBehavior(); // 鼠标摸头：立即醒来
  spawnHearts(2, 1500); // 摸头冒爱心（节流，避免反复进出刷爆）
}

/* ---------- 生命周期 ---------- */
let tickTimer = 0;
let moveBound = false;

onMounted(async () => {
  // 透明窗口：清掉全局背景，仅保留机器人本体（#app 同样被全局样式赋了 --bg-page，需一并清除）
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
  document.getElementById("app")?.style.setProperty("background", "transparent");
  // 过期时钟：success/error 短闪回退 + 空闲行为兜底
  tickTimer = window.setInterval(() => (nowTick.value = Date.now()), 400);
  // pointermove 挂窗口（pointer capture 后事件仍派发到元素，此处兜底）
  window.addEventListener("pointermove", onPointerMove, { passive: true });
  window.addEventListener("keydown", onKeyDown);
  moveBound = true;
  scheduleNext();
  scheduleTip();
  try {
    unlistenState = await listen<{ state: AvatarState }>("pet-state", (e) => applyAiState(e.payload.state));
    unlistenProgress = await listen<PetProgressPayload>("pet-progress", (e) => applyProgress(e.payload));
  } catch {
    /* 非 Tauri 环境忽略 */
  }
});

onBeforeUnmount(() => {
  window.clearTimeout(nextTimer);
  window.clearInterval(tickTimer);
  window.clearTimeout(bubbleTimer);
  window.clearTimeout(tipTimer);
  window.clearTimeout(faceTimer);
  clearBehaviorTimers();
  if (moveBound) {
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("keydown", onKeyDown);
  }
  unlistenState?.();
  unlistenState = null;
  unlistenProgress?.();
  unlistenProgress = null;
});
</script>

<style scoped>
.pet-root {
  position: fixed;
  inset: 0;
  /* 命中区域只保留机器人本体附近，不挡住桌面其他内容 */
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-direction: column;
  cursor: pointer;
  user-select: none;
  -webkit-user-select: none;
}
.pet-anim {
  width: 132px;
  height: 132px;
  margin-bottom: 4px;
}
.pet-anim.pet-hop {
  animation: pet-hop 0.9s ease-out;
}
.pet-anim.pet-wiggle {
  animation: pet-wiggle 1.2s ease-in-out;
}

/* ---------- 气泡 ---------- */
.pet-bubble {
  position: absolute;
  top: 2px;
  left: 50%;
  transform: translateX(-50%);
  max-width: 142px;
  padding: 4px 9px;
  border-radius: 10px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  box-shadow: 0 2px 8px var(--shadow);
  text-align: center;
  z-index: 10;
  pointer-events: none;
}
.pet-bubble::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: -5px;
  width: 8px;
  height: 8px;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  transform: translateX(-50%) rotate(45deg);
}
.bubble-text {
  font-size: 10px;
  line-height: 1.4;
  color: var(--text-body);
  display: block;
}
.bubble-success .bubble-text { color: var(--green); }
.bubble-error .bubble-text { color: var(--red); }

/* 气泡淡入淡出 */
.bubble-enter-active,
.bubble-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}
.bubble-enter-from,
.bubble-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(4px);
}

/* 进度条 */
.pet-progress {
  margin-top: 3px;
  height: 5px;
  border-radius: 3px;
  background: var(--bg-active);
  overflow: hidden;
}
.pet-progress-bar {
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.25s ease;
}
.pet-progress-bar.indeterminate {
  width: 40%;
  animation: progress-slide 1.2s ease-in-out infinite;
}

/* ---------- 爱心 ---------- */
.pet-hearts {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 90px;
  height: 60px;
  pointer-events: none;
  z-index: 9;
}
.heart {
  position: absolute;
  bottom: 0;
  font-size: 13px;
  color: var(--red);
  animation: heart-float 1.3s ease-out forwards;
}

/* ---------- 右键菜单 / 快捷面板 ---------- */
.pet-card {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 6px;
  border-radius: 12px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  box-shadow: 0 4px 16px var(--shadow);
  z-index: 20;
  min-width: 118px;
}
.pet-menu-item {
  appearance: none;
  border: none;
  background: transparent;
  padding: 6px 10px;
  border-radius: 8px;
  font-size: 11px;
  color: var(--text-body);
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}
.pet-menu-item:hover {
  background: var(--bg-hover);
}
/* 快捷功能 emoji 与文字基线对齐 */
.menu-emoji {
  font-size: 13px;
  line-height: 1;
}
/* 快捷功能与全局操作之间的分隔线 */
.pet-menu-divider {
  height: 1px;
  margin: 4px 8px;
  background: var(--border);
}

@keyframes pet-hop {
  0% { transform: translateY(0); }
  35% { transform: translateY(-14px); }
  70% { transform: translateY(3px); }
  100% { transform: translateY(0); }
}
@keyframes pet-wiggle {
  0%, 100% { transform: rotate(0deg); }
  20% { transform: rotate(-4deg); }
  45% { transform: rotate(4deg); }
  70% { transform: rotate(-2.5deg); }
  88% { transform: rotate(2deg); }
}
@keyframes heart-float {
  0% { opacity: 0; transform: translateY(6px) scale(0.6); }
  25% { opacity: 1; }
  100% { opacity: 0; transform: translateY(-38px) scale(1.05); }
}
@keyframes progress-slide {
  0% { margin-left: -40%; }
  100% { margin-left: 100%; }
}
@media (prefers-reduced-motion: reduce) {
  .pet-anim.pet-hop,
  .pet-anim.pet-wiggle,
  .heart,
  .pet-progress-bar.indeterminate {
    animation: none;
  }
}
</style>
