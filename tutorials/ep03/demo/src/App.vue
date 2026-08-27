<template>
  <div class="pet">
    <!-- 打盹时的 Zzz 气泡 -->
    <div v-if="behavior === 'doze'" class="zzz">Z<span>z</span><span>z</span></div>

    <svg viewBox="0 0 120 120" class="robot" :class="{ hop: behavior === 'hop' }">
      <!-- 天线：打盹时指示灯变暗 -->
      <line x1="60" y1="10" x2="60" y2="24" stroke="#8a8f98" stroke-width="4" stroke-linecap="round" />
      <circle cx="60" cy="9" r="5" :fill="behavior === 'doze' ? '#9aa0a6' : '#2080f0'" />
      <!-- 头 -->
      <rect x="22" y="24" width="76" height="56" rx="18" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
      <!-- 屏幕脸 -->
      <rect x="30" y="32" width="60" height="40" rx="10" fill="#f2f4f7" stroke="#e3e5e8" stroke-width="2" />

      <!-- 眼睛：三种画法 —— 打盹闭眼 / 眨眼细线 / 正常圆眼 -->
      <template v-if="eyesClosed">
        <path d="M 43 53 Q 48 57 53 53" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
        <path d="M 67 53 Q 72 57 77 53" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
      </template>
      <template v-else>
        <ellipse :cx="48 + eyeOffset" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
        <ellipse :cx="72 + eyeOffset" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
      </template>

      <!-- 嘴：打盹时是一条直线 -->
      <path
        v-if="behavior === 'doze'"
        d="M 53 64 L 67 64"
        fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round"
      />
      <path v-else d="M 52 63 Q 60 69 68 63" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />

      <!-- 身体 -->
      <rect x="40" y="84" width="40" height="18" rx="9" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { behaviorDuration, nextBehaviorDelay, pickBehavior, type PetBehavior } from "./petBehavior";

// EP02 留下的功课：透明窗口必须清页面背景
document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";

const behavior = ref<PetBehavior>("idle");
const blinking = ref(false);
let behaviorTimer = 0;
let durationTimer = 0;
let blinkTimer = 0;

/** 张望方向：-1 看左、+1 看右 */
const lookDir = ref(0);

const eyesClosed = computed(() => blinking.value || behavior.value === "doze");
const eyeOffset = computed(() => (behavior.value === "lookAround" ? lookDir.value * 4 : 0));

/** 行为调度循环：挑一个行为 → 持续 duration → 歇一会儿 → 再来 */
function scheduleNext() {
  behaviorTimer = window.setTimeout(() => {
    const next = pickBehavior(Math.random());
    if (next !== "idle") {
      behavior.value = next;
      if (next === "lookAround") lookDir.value = Math.random() < 0.5 ? -1 : 1;
      // 行为到期自动回 idle
      durationTimer = window.setTimeout(() => {
        behavior.value = "idle";
        scheduleNext();
      }, behaviorDuration(next));
      return;
    }
    scheduleNext();
  }, nextBehaviorDelay(Math.random()));
}

/** 眨眼是独立的小循环，任何清醒状态下都会发生：每 2.5~5 秒闭眼 150ms */
function scheduleBlink() {
  blinkTimer = window.setTimeout(() => {
    if (behavior.value !== "doze") {
      blinking.value = true;
      window.setTimeout(() => (blinking.value = false), 150);
    }
    scheduleBlink();
  }, 2500 + Math.random() * 2500);
}

onMounted(() => {
  scheduleNext();
  scheduleBlink();
});
onUnmounted(() => {
  clearTimeout(behaviorTimer);
  clearTimeout(durationTimer);
  clearTimeout(blinkTimer);
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
}
.robot {
  width: 132px;
  height: 132px;
  animation: bob 3.6s ease-in-out infinite;
}
/* 跳跃：短暂上浮后落回 */
.robot.hop {
  animation: hop 0.6s ease-out;
}
@keyframes bob {
  0%, 100% { transform: translateY(2px); }
  50% { transform: translateY(-2px); }
}
@keyframes hop {
  0% { transform: translateY(0); }
  40% { transform: translateY(-16px); }
  100% { transform: translateY(0); }
}
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
@keyframes float {
  0%, 100% { transform: translateY(0); opacity: 0.7; }
  50% { transform: translateY(-6px); opacity: 1; }
}
</style>
