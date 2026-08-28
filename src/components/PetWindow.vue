<template>
  <div class="pet-root" :class="{ working }" @contextmenu.prevent="toggleMenu" @click="closeOverlays">
    <!-- 气泡：小贴士 / 任务进度 / 完成与失败反馈 -->
    <Transition name="bubble">
      <div v-if="bubble" class="pet-bubble" :class="`bubble-${bubble.kind}`">
        <template v-if="bubble.kind === 'progress'">
          <span class="bubble-text">{{ t("pet.progress") }}</span>
          <div class="pet-progress">
            <div
              class="pet-progress-bar"
              :class="{ indeterminate: bubble.progress == null }"
              :style="bubble.progress != null ? { transform: `scaleX(${Math.min(bubble.progress, 100) / 100})` } : undefined"
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

    <!-- 装饰流星：空闲时偶尔划过，机器人抬头目送（不可收集） -->
    <span v-if="idleShoot" class="idle-shoot-star" aria-hidden="true">✦</span>
    <!-- 失败安慰：头顶冒一滴汗珠 -->
    <span v-if="sweat" class="sweat-drop" aria-hidden="true"></span>

    <!-- 小行星：机器人的家。任务中加速自转、星环点亮；每完成一个任务收进一颗星 -->
    <div class="pet-planet-scene" :class="{ bounce: planetBounce }" aria-hidden="true">
      <span v-if="shoot" class="shoot-star">✦</span>
      <div class="planet-ring"></div>
      <div class="planet-ball" :class="`stage-${stage}`">
        <div class="planet-surface">
          <span class="crater c1"></span>
          <span class="crater c2"></span>
          <span class="crater c3"></span>
        </div>
        <!-- 二期植物层：随星级进化生长（锚定在可见弧面，不参与自转） -->
        <div v-if="stage > 0" class="planet-flora">
          <span v-if="stage >= 1" class="flora sprout"></span>
          <template v-if="stage >= 2">
            <span class="flora grass g1"></span>
            <span class="flora grass g2"></span>
            <span class="flora mushroom"></span>
          </template>
          <template v-if="stage >= 3">
            <span class="flora flower f1"></span>
            <span class="flora flower f2"></span>
          </template>
        </div>
      </div>
      <span v-if="starCount > 0" class="planet-badge">✦ {{ starCount }}</span>
    </div>

    <!-- 二期轨道串门：两颗邻居小行星，点击可把机器人叫回家 -->
    <button class="orbit-planet left" type="button" :aria-label="t('pet.callBack')" @click.stop="callHome"></button>
    <button class="orbit-planet right" type="button" :aria-label="t('pet.callBack')" @click.stop="callHome"></button>

    <!-- 包装层：空闲行为与戳一戳的小跳/摇摆动画（不进头像组件，避免污染状态语义） -->
    <div
      class="pet-anim"
      :class="[wrapperAnim, travelAnim, `spot-${spot}`, { dozing }]"
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
      <!-- 昼夜节律：夜里给机器人戴上睡帽 -->
      <span v-if="nightCap" class="night-cap" aria-hidden="true">
        <span class="cap-body"></span>
        <span class="cap-pom"></span>
      </span>
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
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import AssistantAvatar, { type AvatarState } from "./AssistantAvatar.vue";
import { petHide, petOpenMain } from "../api";
import type { PetProgressPayload } from "../utils/petProgress";
import {
  aiStateHoldMs, dayPhaseOf, nextBehaviorDelay, nextShootDelay, nextTipDelay,
  nextVisitDelay, pickBehavior, pickPokeReaction, pickVisitSide,
  resolveDisplayState, visitDwellMs,
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

/** 行星是否在工作中：任务进行中自转加速、星环点亮 */
const working = computed(() => bubble.value?.kind === "progress");

/* ---------- 任务进度反馈（主窗口 usePanelTask / history 广播） ---------- */
let unlistenProgress: UnlistenFn | null = null;

function applyProgress(p: PetProgressPayload) {
  interruptBehavior();
  faceOverride.value = null;
  switch (p.phase) {
    case "start":
      if (spot.value !== "home" || travelAnim.value) callHome(); // 开工了：串门中立刻回家
      showBubble({ kind: "progress", text: "", progress: p.progress }, null);
      break;
    case "tick":
      if (bubble.value?.kind === "progress") bubble.value.progress = p.progress;
      break;
    case "done":
      collectStar();
      // 机器人蹦起来接星，落地时把星球压得 Q 弹一下（先跳后压，时序错开）
      wrapperAnim.value = "pet-hop";
      window.setTimeout(() => (wrapperAnim.value = ""), 900);
      window.setTimeout(bouncePlanet, 420);
      showBubble(
        {
          kind: "success",
          text: starCount.value % 10 === 0
            ? t("pet.starMilestone", { n: starCount.value })
            : t("pet.starCollected", { n: starCount.value }),
        },
        2500,
      );
      setFaceOverride("success", 2500);
      break;
    case "error":
      // 安慰动作：垂头丧气 + 冒汗珠（文案由 errorMsg 气泡承担）
      wrapperAnim.value = "pet-sad";
      sweat.value = true;
      window.setTimeout(() => (wrapperAnim.value = ""), 2600);
      window.setTimeout(() => (sweat.value = false), 1800);
      showBubble({ kind: "error", text: t("pet.errorMsg") }, 3200);
      setFaceOverride("error", 3200);
      break;
  }
}

/* ---------- 空闲行为调度器：张望 / 打盹 / 小跳 / 摇摆 / 伸懒腰 ---------- */
const lookShift = ref<{ x: number; y: number } | null>(null);
const wrapperAnim = ref<"" | "pet-hop" | "pet-wiggle" | "pet-stretch" | "pet-gaze" | "pet-sad">("");
/** 失败安慰的汗珠（头顶冒一滴，配合垂头动作） */
const sweat = ref(false);
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
  // 串门中/跳途中不做空闲行为，5s 后再试（回家后才继续卖萌）
  if (spot.value !== "home" || travelAnim.value) {
    behaviorTimers.push(window.setTimeout(scheduleNext, 5000));
    return;
  }
  const b = pickBehavior(Math.random(), phase.value);
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
      later(b.duration * 0.5, bouncePlanet); // 落地踩得星球弹一下（延迟约为跳起的落点）
      later(b.duration, () => (wrapperAnim.value = ""));
      break;
    case "wiggle":
      wrapperAnim.value = "pet-wiggle";
      later(b.duration, () => (wrapperAnim.value = ""));
      break;
    case "stretch":
      wrapperAnim.value = "pet-stretch";
      later(b.duration, () => (wrapperAnim.value = ""));
      break;
  }
  later(b.duration, scheduleNext);
}

/* ---------- 装饰流星：空闲时偶尔划过天际，机器人抬头目送（只观赏不收集） ---------- */
let shootSchedTimer = 0;
const idleShoot = ref(false);
let idleShootTimer = 0;

function scheduleIdleShoot() {
  window.clearTimeout(shootSchedTimer);
  shootSchedTimer = window.setTimeout(tryIdleShoot, nextShootDelay(Math.random()));
}

function tryIdleShoot() {
  // 只在家、不在途、空闲、无气泡/菜单时望星；否则稍后再试（链不中断）
  const canGaze =
    spot.value === "home" && !travelAnim.value && !working.value &&
    !bubble.value && !menuOpen.value && !aiActive.value;
  if (canGaze) {
    cancelBehavior(); // 让位：清掉进行中的空闲行为，望星结束后重新排链
    idleShoot.value = true;
    idleShootTimer = window.setTimeout(() => (idleShoot.value = false), 1800);
    lookShift.value = { x: 0, y: -7 }; // 瞳孔上瞟追踪流星
    wrapperAnim.value = "pet-gaze";
    behaviorTimers.push(window.setTimeout(() => {
      lookShift.value = null;
      wrapperAnim.value = "";
      scheduleNext(); // 续上空闲行为链（cancelBehavior 不会自动重排）
    }, 2000));
  }
  scheduleIdleShoot();
}

/* ---------- 昼夜节律：早晨伸懒腰问早安，夜里戴睡帽道晚安、更容易打盹 ---------- */
const phase = computed(() => dayPhaseOf(new Date(nowTick.value).getHours()));
const nightCap = computed(() => phase.value === "night");

function greetMorning() {
  interruptBehavior();
  wrapperAnim.value = "pet-stretch";
  window.setTimeout(() => (wrapperAnim.value = ""), 1600);
  showBubble({ kind: "tip", text: t("pet.morningGreeting") }, 3200);
}

watch(phase, (p) => {
  if (p === "morning") greetMorning();
  else if (p === "night") showBubble({ kind: "tip", text: t("pet.nightGreeting") }, 3200);
  // 白天段静默过渡，不播报（避免刷屏）
});

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

/* ---------- 小行星：任务星星收集（持久化在 localStorage） ---------- */
const starCount = ref<number>(Number.parseInt(localStorage.getItem("docmorph-pet-stars") ?? "0", 10) || 0);
const shoot = ref(false);
let shootTimer = 0;
watch(starCount, (n) => localStorage.setItem("docmorph-pet-stars", String(n)));

/** 机器人落地时星球的 Q 弹下压 */
const planetBounce = ref(false);
let bounceTimer = 0;

function bouncePlanet() {
  window.clearTimeout(bounceTimer);
  planetBounce.value = true;
  bounceTimer = window.setTimeout(() => (planetBounce.value = false), 600);
}

function collectStar() {
  starCount.value += 1;
  window.clearTimeout(shootTimer);
  shoot.value = true;
  shootTimer = window.setTimeout(() => (shoot.value = false), 950);
}

/* ---------- 二期能量星球：星级越多越进化（岩石 → 嫩芽 → 草地 → 开花） ---------- */
const stage = computed(() => (starCount.value >= 20 ? 3 : starCount.value >= 10 ? 2 : starCount.value >= 5 ? 1 : 0));
let prevStage = stage.value; // 初始档位不播报，只在升档时庆祝
watch(stage, (s) => {
  if (s > prevStage) {
    showBubble({ kind: "tip", text: t("pet.stageUp") }, 3200);
    spawnHearts(3);
  }
  prevStage = s;
});

/* ---------- 二期轨道串门：空闲时去邻居星球，点击邻居/戳一戳/开工都能叫回家 ---------- */
type Spot = "home" | "left" | "right";
type TravelAnim = "" | "travel-out-left" | "travel-out-right" | "travel-back-left" | "travel-back-right";
const spot = ref<Spot>("home");
const travelAnim = ref<TravelAnim>("");
let travelTimer = 0;
let visitTimer = 0;
let dwellTimer = 0;

function travelTo(side: "left" | "right") {
  if (spot.value !== "home" || travelAnim.value) return;
  cancelBehavior();
  travelAnim.value = side === "left" ? "travel-out-left" : "travel-out-right";
  travelTimer = window.setTimeout(() => {
    spot.value = side;
    travelAnim.value = "";
    dwellTimer = window.setTimeout(callHome, visitDwellMs(Math.random())); // 逗留一阵自己回家
  }, 900);
}

function callHome() {
  window.clearTimeout(dwellTimer);
  if (travelAnim.value) {
    // 跳途中被叫：任务/主人优先，直接切回不播完动画
    window.clearTimeout(travelTimer);
    travelAnim.value = "";
    spot.value = "home";
    scheduleVisit();
    return;
  }
  if (spot.value === "home") {
    scheduleVisit();
    return;
  }
  travelAnim.value = spot.value === "left" ? "travel-back-left" : "travel-back-right";
  travelTimer = window.setTimeout(() => {
    spot.value = "home";
    travelAnim.value = "";
    scheduleVisit();
  }, 900);
}

function scheduleVisit() {
  window.clearTimeout(visitTimer);
  visitTimer = window.setTimeout(tryVisit, nextVisitDelay(Math.random()));
}

function tryVisit() {
  // 只在家、不在途、不干活、无菜单、不打盹时出门
  if (spot.value === "home" && !travelAnim.value && !working.value && !menuOpen.value && !dozing.value) {
    travelTo(pickVisitSide(Math.random()));
    return;
  }
  visitTimer = window.setTimeout(tryVisit, 20_000); // 条件不满足，稍后再试
}

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
  // 不在家：戳一戳 = 叫回家（不玩反应）
  if (spot.value !== "home" || travelAnim.value) {
    callHome();
    return;
  }
  interruptBehavior();
  faceOverride.value = null;
  const r = pickPokeReaction(Math.random());
  switch (r) {
    case "hop":
      wrapperAnim.value = "pet-hop";
      window.setTimeout(bouncePlanet, 450);
      window.setTimeout(() => (wrapperAnim.value = ""), 900);
      break;
    case "wiggle":
      wrapperAnim.value = "pet-wiggle";
      window.setTimeout(() => (wrapperAnim.value = ""), 1200);
      break;
    case "hearts":
      spawnHearts(3);
      break;
  }
}

function onHoverEnter() {
  if (dozing.value) interruptBehavior(); // 鼠标摸头：立即醒来
  spawnHearts(1, 5000); // 摸头冒爱心（单颗 + 长节流，避免反复进出刷屏）
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
  scheduleVisit();
  scheduleIdleShoot();
  // 启动即处于早/夜段：当次会话问候一句（跨段切换由 watch 接管）
  window.setTimeout(() => {
    if (phase.value === "morning") greetMorning();
    else if (phase.value === "night") showBubble({ kind: "tip", text: t("pet.nightGreeting") }, 3200);
  }, 1200);
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
  window.clearTimeout(shootTimer);
  window.clearTimeout(bounceTimer);
  window.clearTimeout(travelTimer);
  window.clearTimeout(visitTimer);
  window.clearTimeout(dwellTimer);
  window.clearTimeout(shootSchedTimer);
  window.clearTimeout(idleShootTimer);
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
  position: relative;
  width: 132px;
  height: 132px;
  margin-bottom: 30px; /* 脚踩在小行星的弧面上 */
}
.pet-anim.pet-hop {
  animation: pet-hop 0.9s ease-out;
}
.pet-anim.pet-wiggle {
  animation: pet-wiggle 1.2s ease-in-out;
}
.pet-anim.pet-stretch {
  animation: pet-stretch 1.6s ease-in-out;
  transform-origin: bottom center; /* 从脚底向上抻开，像真的在伸懒腰 */
}
.pet-anim.pet-gaze {
  animation: pet-gaze 2s ease-in-out;
  transform-origin: bottom center;
}
.pet-anim.pet-sad {
  /* 失败安慰：垂头丧气歪一下，配合汗珠与安慰文案 */
  animation: pet-sad 2.6s ease-in-out;
  transform-origin: bottom center;
}
.pet-anim.dozing {
  /* 打盹时歪向小行星，像靠着它睡着了 */
  transform: rotate(-4deg) translateY(3px);
  transform-origin: bottom center;
  transition: transform 0.8s ease;
}

/* ---------- 小行星 ---------- */
.pet-planet-scene {
  position: absolute;
  left: 50%;
  bottom: -6px;
  width: 130px;
  height: 70px;
  transform: translateX(-50%);
  pointer-events: none; /* 纯装饰：不扩大可点区域 */
}
.planet-ball {
  position: absolute;
  left: 50%;
  bottom: -72px; /* 圆大部分沉出窗口，只露弧面 */
  width: 130px;
  height: 130px;
  border-radius: 50%;
  transform: translateX(-50%);
  background: radial-gradient(circle at 38% 16%, #8fb9ff 0%, #5a7edc 55%, #3a529e 100%);
  overflow: hidden;
  box-shadow: 0 -1px 0 rgba(255, 255, 255, 0.35) inset;
}
.planet-surface {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  animation: planet-idle-spin 80s linear infinite;
}
.working .planet-surface {
  animation: planet-work-spin 7s linear infinite; /* 干活时加速自转 */
}
.crater {
  position: absolute;
  border-radius: 50%;
  background: rgba(26, 42, 96, 0.35);
}
.crater.c1 { width: 16px; height: 16px; left: 26px; top: 22px; }
.crater.c2 { width: 10px; height: 10px; left: 78px; top: 30px; }
.crater.c3 { width: 13px; height: 13px; left: 52px; top: 62px; }

/* ---------- 二期：能量星球进化（岩石 → 嫩芽 → 草地 → 开花） ---------- */
.planet-ball.stage-0 { background: radial-gradient(circle at 38% 16%, #b9bec9 0%, #8b91a0 55%, #5c6270 100%); }
.planet-ball.stage-1 { background: radial-gradient(circle at 38% 16%, #cdc39a 0%, #a39a6d 55%, #6e674a 100%); }
.planet-ball.stage-2 { background: radial-gradient(circle at 38% 16%, #a8d68a 0%, #6ea85f 55%, #43703f 100%); }
.planet-ball.stage-3 { background: radial-gradient(circle at 38% 16%, #c2eaa6 0%, #79c56d 55%, #3f8a4a 100%); }
/* 植物长出来后陨石坑淡出 */
.planet-ball.stage-2 .crater,
.planet-ball.stage-3 .crater { opacity: 0.35; }

.planet-flora { position: absolute; inset: 0; border-radius: 50%; pointer-events: none; }
.flora { position: absolute; display: block; }
.flora.sprout { left: 63px; top: 3px; width: 3px; height: 11px; background: #4c8b3f; border-radius: 2px; }
.flora.sprout::before {
  content: "";
  position: absolute;
  left: -5px;
  top: 1px;
  width: 8px;
  height: 5px;
  background: #5fae4d;
  border-radius: 50% 50% 20% 50%;
  transform: rotate(-28deg);
}
.flora.grass { width: 3px; height: 8px; background: #3f7a37; border-radius: 3px 3px 0 0; }
.flora.grass.g1 { left: 40px; top: 9px; transform: rotate(-10deg); }
.flora.grass.g2 { left: 88px; top: 10px; transform: rotate(12deg); }
.flora.mushroom { left: 76px; top: 5px; width: 9px; height: 6px; background: #e2574c; border-radius: 9px 9px 2px 2px; }
.flora.mushroom::after {
  content: "";
  position: absolute;
  left: 3px;
  top: 5px;
  width: 3px;
  height: 5px;
  background: #f0e2c8;
  border-radius: 1px;
}
.flora.flower { width: 7px; height: 7px; border-radius: 50%; }
.flora.flower.f1 { left: 50px; top: 4px; background: #f7ba2a; }
.flora.flower.f2 { left: 92px; top: 7px; background: #f27fa5; }

/* ---------- 二期：邻居星球（轨道串门） ---------- */
.orbit-planet {
  position: absolute;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: none;
  padding: 0;
  cursor: pointer;
  z-index: 8;
  animation: orbit-bob 4s ease-in-out infinite;
}
.orbit-planet.left { left: 10px; top: 54px; background: radial-gradient(circle at 35% 30%, #ffd9a0, #e8965a 70%); }
.orbit-planet.right { right: 10px; top: 62px; background: radial-gradient(circle at 35% 30%, #cfe3ff, #7d9fe8 70%); animation-delay: -2s; }
.orbit-planet:hover { filter: brightness(1.25); }

/* 机器人在邻居星球上的驻留位（跳跃动画结束后的落点） */
.pet-anim.spot-left { transform: translateX(-20px); }
.pet-anim.spot-right { transform: translateX(20px); }
.pet-anim.travel-out-left { animation: travel-out-left 0.9s ease-in-out forwards; }
.pet-anim.travel-out-right { animation: travel-out-right 0.9s ease-in-out forwards; }
.pet-anim.travel-back-left { animation: travel-back-left 0.9s ease-in-out forwards; }
.pet-anim.travel-back-right { animation: travel-back-right 0.9s ease-in-out forwards; }
.planet-ring {
  position: absolute;
  left: 50%;
  bottom: -16px;
  width: 176px;
  height: 40px;
  border-radius: 50%;
  border: 2px solid var(--border);
  opacity: 0.55;
  transform: translateX(-50%) rotate(-7deg);
  clip-path: inset(50% 0 0 0); /* 只显示下半段：视觉上绕到星球背面 */
  transition: border-color 0.3s ease, opacity 0.3s ease;
}
.working .planet-ring {
  border-color: var(--accent);
  opacity: 0.95; /* 干活时星环点亮 */
}
.planet-badge {
  position: absolute;
  left: 50%;
  bottom: 1px;
  transform: translateX(-50%);
  font-size: 9px;
  line-height: 1;
  padding: 2px 7px;
  border-radius: 8px;
  color: var(--text-body);
  background: var(--bg-panel);
  border: 1px solid var(--border);
  white-space: nowrap;
}
.shoot-star {
  position: absolute;
  left: 12px;
  top: -52px;
  font-size: 15px;
  color: var(--yellow, #f7ba2a);
  z-index: 11;
  animation: star-collect 0.9s ease-in forwards;
}
.pet-planet-scene.bounce .planet-ball {
  animation: planet-bounce 0.55s ease-out;
  transform-origin: bottom center;
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
  /* scaleX 只走合成器，不触发布局；左对齐保证从头部增长 */
  transform-origin: left;
  transition: transform 0.25s ease;
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

/* ---------- 装饰流星 / 汗珠 ---------- */
.idle-shoot-star {
  position: absolute;
  top: 24px;
  left: 0;
  font-size: 12px;
  color: var(--yellow, #f7ba2a);
  z-index: 6;
  pointer-events: none;
  animation: idle-shoot 1.8s ease-in forwards;
}
.sweat-drop {
  position: absolute;
  left: 50%;
  top: 46px;
  width: 7px;
  height: 9px;
  margin-left: 30px;
  border-radius: 50% 50% 50% 50% / 38% 38% 62% 62%;
  background: #7fb8f0;
  pointer-events: none;
  z-index: 9;
  animation: sweat-slide 1.8s ease-in forwards;
}

/* ---------- 昼夜节律：夜里的睡帽 ---------- */
.night-cap {
  position: absolute;
  top: 14px;
  left: 50%;
  width: 60px;
  height: 30px;
  transform: translateX(-50%) rotate(-6deg);
  pointer-events: none;
  z-index: 3;
  animation: cap-appear 0.6s ease-out;
}
.cap-body {
  position: absolute;
  inset: 0;
  background: linear-gradient(115deg, #93a9e8, #6b83cf);
  border-radius: 30px 30px 8px 8px;
}
.cap-body::after {
  /* 帽檐白色滚边 */
  content: "";
  position: absolute;
  left: -2px;
  right: -2px;
  bottom: 0;
  height: 9px;
  border-radius: 5px;
  background: #dfe6fb;
}
.cap-pom {
  position: absolute;
  top: -6px;
  left: 50%;
  width: 10px;
  height: 10px;
  margin-left: -5px;
  border-radius: 50%;
  background: #f5f7ff;
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
  0% { transform: translateX(-100%); }
  100% { transform: translateX(250%); }
}
@keyframes planet-idle-spin {
  to { transform: rotate(360deg); }
}
@keyframes planet-work-spin {
  to { transform: rotate(360deg); }
}
@keyframes star-collect {
  0% { opacity: 0; transform: translate(-34px, -26px) scale(0.5); }
  30% { opacity: 1; }
  100% { opacity: 0; transform: translate(52px, 66px) scale(1.15); }
}
@keyframes planet-bounce {
  /* keyframes 内自包含 translateX(-50%)，避免覆盖球体定位 */
  0% { transform: translateX(-50%) scale(1); }
  30% { transform: translateX(-50%) scale(1.06, 0.92); }
  60% { transform: translateX(-50%) scale(0.98, 1.03); }
  100% { transform: translateX(-50%) scale(1); }
}
/* 串门：抛物线小跳（外层位移自包含，结束即落到驻留位） */
@keyframes travel-out-left {
  0% { transform: translate(0, 0); }
  50% { transform: translate(-10px, -16px); }
  100% { transform: translate(-20px, 0); }
}
@keyframes travel-out-right {
  0% { transform: translate(0, 0); }
  50% { transform: translate(10px, -16px); }
  100% { transform: translate(20px, 0); }
}
@keyframes travel-back-left {
  0% { transform: translate(-20px, 0); }
  50% { transform: translate(-10px, -16px); }
  100% { transform: translate(0, 0); }
}
@keyframes travel-back-right {
  0% { transform: translate(20px, 0); }
  50% { transform: translate(10px, -16px); }
  100% { transform: translate(0, 0); }
}
@keyframes orbit-bob {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-5px); }
}
@keyframes pet-stretch {
  0%, 100% { transform: scale(1); }
  32% { transform: scale(1.06, 1.12) translateY(-4px); }
  58% { transform: scale(0.98, 0.95) translateY(1px); }
  80% { transform: scale(1.02, 1.04); }
}
@keyframes pet-gaze {
  0%, 100% { transform: translateY(0) rotate(0deg); }
  25%, 72% { transform: translateY(-3px) rotate(-4deg); }
}
@keyframes pet-sad {
  0%, 100% { transform: translateY(0) rotate(0deg); }
  22%, 70% { transform: translateY(3px) rotate(5deg); }
}
@keyframes idle-shoot {
  0% { opacity: 0; transform: translate(-6px, 8px); }
  22% { opacity: 1; }
  80% { opacity: 0.9; }
  100% { opacity: 0; transform: translate(150px, -16px); }
}
@keyframes sweat-slide {
  0% { opacity: 0; transform: translateY(0) scale(0.7); }
  20% { opacity: 1; transform: translateY(2px) scale(1); }
  100% { opacity: 0; transform: translateY(16px) scale(0.9); }
}
@keyframes cap-appear {
  0% { opacity: 0; transform: translateX(-50%) rotate(-6deg) translateY(-8px); }
  100% { opacity: 1; transform: translateX(-50%) rotate(-6deg) translateY(0); }
}
@media (prefers-reduced-motion: reduce) {
  .pet-anim.pet-hop,
  .pet-anim.pet-wiggle,
  .pet-anim.pet-stretch,
  .pet-anim.pet-gaze,
  .pet-anim.pet-sad,
  .heart,
  .idle-shoot-star,
  .sweat-drop,
  .night-cap,
  .pet-progress-bar.indeterminate,
  .planet-surface,
  .pet-planet-scene.bounce .planet-ball,
  .orbit-planet,
  .pet-anim.travel-out-left,
  .pet-anim.travel-out-right,
  .pet-anim.travel-back-left,
  .pet-anim.travel-back-right,
  .shoot-star {
    animation: none;
  }
}
</style>
