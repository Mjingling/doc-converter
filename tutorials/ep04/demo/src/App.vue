<template>
  <div
    class="pet"
    @mousedown.left="startDrag"
    @click="onPoke"
    @mouseenter="onHover"
    @mousemove="onHoverMove"
    @contextmenu.prevent="openMenu"
  >
    <!-- 气泡：戳一戳的台词 -->
    <div v-if="bubble" class="bubble">{{ bubble }}</div>

    <!-- 打盹时的 Zzz -->
    <div v-if="behavior === 'doze'" class="zzz">Z<span>z</span><span>z</span></div>

    <!-- 爱心粒子 -->
    <span
      v-for="h in hearts"
      :key="h.id"
      class="heart"
      :style="{ left: h.x + 'px', top: h.y + 'px' }"
    >♥</span>

    <svg
      viewBox="0 0 120 120"
      class="robot"
      :class="{ hop: reaction === 'hop', wiggle: reaction === 'wiggle' }"
    >
      <line x1="60" y1="10" x2="60" y2="24" stroke="#8a8f98" stroke-width="4" stroke-linecap="round" />
      <circle cx="60" cy="9" r="5" :fill="behavior === 'doze' ? '#9aa0a6' : '#2080f0'" />
      <rect x="22" y="24" width="76" height="56" rx="18" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
      <rect x="30" y="32" width="60" height="40" rx="10" fill="#f2f4f7" stroke="#e3e5e8" stroke-width="2" />
      <template v-if="eyesClosed">
        <path d="M 43 53 Q 48 57 53 53" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
        <path d="M 67 53 Q 72 57 77 53" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
      </template>
      <template v-else>
        <ellipse :cx="48 + eyeOffset" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
        <ellipse :cx="72 + eyeOffset" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
      </template>
      <path
        v-if="behavior === 'doze'"
        d="M 53 64 L 67 64"
        fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round"
      />
      <path v-else d="M 52 63 Q 60 69 68 63" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
      <rect x="40" y="84" width="40" height="18" rx="9" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
    </svg>

    <!-- 右键菜单：普通 HTML 绝对定位，比系统菜单好控制样式 -->
    <ul v-if="menu.open" class="menu" :style="{ left: menu.x + 'px', top: menu.y + 'px' }" @click.stop>
      <li @click="pokeFromMenu">再戳一下 👉</li>
      <li @click="hidePet">隐藏宠物 🙈</li>
      <li class="menu-footer">桌宠教程 EP04</li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  behaviorDuration,
  nextBehaviorDelay,
  pickBehavior,
  pickPokeLine,
  pickPokeReaction,
  type PetBehavior,
  type PokeReaction,
} from "./petBehavior";

document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";

// ── EP03 的行为引擎（原样保留）─────────────────────────────
const behavior = ref<PetBehavior>("idle");
const blinking = ref(false);
const lookDir = ref(0);
let behaviorTimer = 0;
let durationTimer = 0;
let blinkTimer = 0;

const eyesClosed = computed(() => blinking.value || behavior.value === "doze");
const eyeOffset = computed(() => (behavior.value === "lookAround" ? lookDir.value * 4 : 0));

function scheduleNext() {
  behaviorTimer = window.setTimeout(() => {
    const next = pickBehavior(Math.random());
    if (next !== "idle") {
      behavior.value = next;
      if (next === "lookAround") lookDir.value = Math.random() < 0.5 ? -1 : 1;
      durationTimer = window.setTimeout(() => {
        behavior.value = "idle";
        scheduleNext();
      }, behaviorDuration(next));
      return;
    }
    scheduleNext();
  }, nextBehaviorDelay(Math.random()));
}

function scheduleBlink() {
  blinkTimer = window.setTimeout(() => {
    if (behavior.value !== "doze") {
      blinking.value = true;
      window.setTimeout(() => (blinking.value = false), 150);
    }
    scheduleBlink();
  }, 2500 + Math.random() * 2500);
}

// ── EP04 新增：戳一戳 ────────────────────────────────────
const reaction = ref<PokeReaction | null>(null);
const bubble = ref("");
let bubbleTimer = 0;
let reactionTimer = 0;

function poke() {
  behavior.value = "idle"; // 戳醒打盹中的宠物
  reaction.value = pickPokeReaction(Math.random());
  if (reaction.value === "hearts") {
    // hearts 反应：连冒三颗爱心（临时解除节流）
    lastHeartAt = 0;
    spawnHeart();
    window.setTimeout(spawnHeart, 150);
    window.setTimeout(spawnHeart, 300);
  }
  bubble.value = pickPokeLine(Math.random());
  clearTimeout(bubbleTimer);
  bubbleTimer = window.setTimeout(() => (bubble.value = ""), 2200);
  clearTimeout(reactionTimer);
  reactionTimer = window.setTimeout(() => (reaction.value = null), 700);
}

function onPoke() {
  // 拖动窗口结束时也可能触发 click，Tauri 拖拽不会派发，可放心处理
  poke();
}

// ── EP04 新增：悬停摸头冒爱心（400ms 节流）────────────────
interface Heart { id: number; x: number; y: number }
const hearts = ref<Heart[]>([]);
let heartSeq = 0;
let lastHeartAt = 0;

function onHover() {
  spawnHeart();
}
function onHoverMove() {
  spawnHeart();
}
function spawnHeart() {
  const now = Date.now();
  if (now - lastHeartAt < 400) return;
  lastHeartAt = now;
  const h: Heart = { id: heartSeq++, x: 30 + Math.random() * 80, y: 20 + Math.random() * 40 };
  hearts.value.push(h);
  // 动画结束后移除，避免数组无限增长
  window.setTimeout(() => {
    hearts.value = hearts.value.filter((x) => x.id !== h.id);
  }, 1200);
}

// ── EP04 新增：拖动窗口 ──────────────────────────────────
async function startDrag() {
  // 交给操作系统做窗口拖拽；一旦进入拖拽，本次不会再触发 click
  try {
    await getCurrentWindow().startDragging();
  } catch {
    /* 权限缺失时静默降级 */
  }
}

// ── EP04 新增：右键菜单 ──────────────────────────────────
const menu = reactive({ open: false, x: 0, y: 0 });

function openMenu(e: MouseEvent) {
  // 窗口只有 150px 宽，菜单贴左侧放，避免溢出
  menu.x = Math.min(e.clientX, 40);
  menu.y = Math.min(e.clientY, 90);
  menu.open = true;
}
function closeMenu() {
  menu.open = false;
}
function pokeFromMenu() {
  closeMenu();
  poke();
}
async function hidePet() {
  closeMenu();
  await getCurrentWindow().hide(); // 隐藏窗口，进程仍在（教程到此为止）
}

// 点击空白处关掉菜单
function onWindowClick() {
  if (menu.open) closeMenu();
}

onMounted(() => {
  scheduleNext();
  scheduleBlink();
  window.addEventListener("click", onWindowClick);
});
onUnmounted(() => {
  clearTimeout(behaviorTimer);
  clearTimeout(durationTimer);
  clearTimeout(blinkTimer);
  clearTimeout(bubbleTimer);
  clearTimeout(reactionTimer);
  window.removeEventListener("click", onWindowClick);
});
</script>

<style scoped>
.pet {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  padding-bottom: 8px;
  user-select: none;
  cursor: grab;
}
.robot {
  width: 132px;
  height: 132px;
  animation: bob 3.6s ease-in-out infinite;
  pointer-events: none; /* 事件统一由 .pet 接收，避免拖动/点击打架 */
}
.robot.hop { animation: hop 0.6s ease-out; }
.robot.wiggle { animation: wiggle 0.55s ease-in-out; }

.bubble {
  position: absolute;
  top: 6px;
  left: 50%;
  transform: translateX(-50%);
  max-width: 130px;
  padding: 5px 10px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #e3e5e8;
  font-size: 12px;
  color: #1a1a1a;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  white-space: nowrap;
}

.heart {
  position: absolute;
  color: #f0547d;
  font-size: 14px;
  pointer-events: none;
  animation: heart-up 1.1s ease-out forwards;
}

.menu {
  position: absolute;
  z-index: 10;
  margin: 0;
  padding: 4px;
  list-style: none;
  min-width: 120px;
  background: rgba(255, 255, 255, 0.97);
  border: 1px solid #e3e5e8;
  border-radius: 10px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
  font-size: 13px;
  cursor: default;
}
.menu li {
  padding: 7px 12px;
  border-radius: 6px;
  cursor: pointer;
}
.menu li:hover { background: #f0f4ff; }
.menu .menu-footer {
  color: #8a8f98;
  font-size: 11px;
  cursor: default;
}
.menu .menu-footer:hover { background: transparent; }

.zzz {
  position: absolute;
  top: 18px;
  right: 18px;
  font-size: 20px;
  font-weight: 700;
  color: #2080f0;
  animation: float 2.4s ease-in-out infinite;
}
.zzz span:first-child { font-size: 16px; }
.zzz span:last-child { font-size: 12px; }

@keyframes bob {
  0%, 100% { transform: translateY(2px); }
  50% { transform: translateY(-2px); }
}
@keyframes hop {
  0% { transform: translateY(0); }
  40% { transform: translateY(-16px); }
  100% { transform: translateY(0); }
}
@keyframes wiggle {
  0%, 100% { transform: rotate(0); }
  25% { transform: rotate(-8deg); }
  75% { transform: rotate(8deg); }
}
@keyframes float {
  0%, 100% { transform: translateY(0); opacity: 0.7; }
  50% { transform: translateY(-6px); opacity: 1; }
}
@keyframes heart-up {
  0% { transform: translateY(0) scale(0.8); opacity: 0; }
  20% { opacity: 1; }
  100% { transform: translateY(-34px) scale(1.25); opacity: 0; }
}
</style>
