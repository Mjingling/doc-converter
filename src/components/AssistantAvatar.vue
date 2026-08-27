<template>
  <div
    ref="rootEl"
    class="avatar"
    :class="[`size-${size}`, `state-${state}`]"
    :style="{ '--state-color': stateColor }"
    aria-hidden="true"
  >
    <!-- 光晕（仅大尺寸；颜色跟随状态） -->
    <div v-if="size === 'lg'" class="halo"></div>

    <!-- 头部整体：悬浮 / 成功弹跳 / 出错蔫蔫下沉 -->
    <div class="bob" :class="{ hop: state === 'success', droop: state === 'error' }">
      <svg viewBox="0 0 240 240" class="face">
        <!-- 天线：杆 + 状态灯球 -->
        <line x1="120" y1="26" x2="120" y2="48" class="stem" />
        <g v-if="state === 'working'" class="ring-wrap">
          <circle cx="120" cy="16" r="15" class="ring" />
        </g>
        <circle cx="120" cy="16" r="9" class="ball" />

        <!-- 思考点（等待模型时头顶三颗跳动） -->
        <g v-if="state === 'thinking'" class="think-dots">
          <circle cx="168" cy="44" r="4" />
          <circle cx="182" cy="34" r="4" />
          <circle cx="196" cy="24" r="4" />
        </g>

        <!-- 打盹 Zzz 气泡（依次上浮淡出） -->
        <g v-if="state === 'dozing'" class="zzz">
          <text x="164" y="56" class="zzz-t s">z</text>
          <text x="180" y="42" class="zzz-t m">z</text>
          <text x="198" y="28" class="zzz-t l">Z</text>
        </g>

        <!-- 头：外壳 + 屏幕脸；出错时轻微歪头 -->
        <g class="head" :class="{ tilt: state === 'error' }">
          <rect x="46" y="46" width="148" height="110" rx="34" class="shell" />
          <rect x="62" y="62" width="116" height="78" rx="18" class="screen" />

          <!-- 眼睛：外层跟随鼠标位移（仅 idle），内层眨眼/眯眼/下视 -->
          <g class="eyes-track" :style="trackStyle">
            <g class="eyes" :class="{ blink: blinking, 'look-down': state === 'working' }">
              <!-- 成功：^ ^ 弧线眼 -->
              <template v-if="state === 'success'">
                <path d="M 86 106 Q 98 88 110 106" class="eye-stroke" />
                <path d="M 130 106 Q 142 88 154 106" class="eye-stroke" />
              </template>
              <!-- 出错：× × 眼 -->
              <template v-else-if="state === 'error'">
                <g class="x-eye">
                  <line x1="89" y1="90" x2="107" y2="108" />
                  <line x1="89" y1="108" x2="107" y2="90" />
                </g>
                <g class="x-eye">
                  <line x1="133" y1="90" x2="151" y2="108" />
                  <line x1="133" y1="108" x2="151" y2="90" />
                </g>
              </template>
              <!-- 打盹：闭眼横线 -->
              <template v-else-if="state === 'dozing'">
                <line x1="88" y1="100" x2="108" y2="100" class="eye-stroke" />
                <line x1="132" y1="100" x2="152" y2="100" class="eye-stroke" />
              </template>
              <!-- 其余状态：胶囊眼（thinking 眯成扁条） -->
              <template v-else>
                <ellipse cx="98" cy="100" rx="8" :ry="state === 'thinking' ? 6 : 15" class="eye" />
                <ellipse cx="142" cy="100" rx="8" :ry="state === 'thinking' ? 6 : 15" class="eye" />
              </template>
            </g>
          </g>
        </g>

        <!-- 身体（仅大尺寸露一角，增加悬浮感） -->
        <g v-if="size === 'lg'" class="body">
          <rect x="78" y="160" width="84" height="34" rx="15" class="shell" />
        </g>
      </svg>
    </div>

    <!-- 底部投影（仅大尺寸，不随头部浮动） -->
    <div v-if="size === 'lg'" class="shadow"></div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

/** 头像状态：idle 待机 / thinking 等待模型 / working 执行工具 / success 完成 / error 出错 / dozing 打盹 */
export type AvatarState = "idle" | "thinking" | "working" | "success" | "error" | "dozing";

const props = withDefaults(
  defineProps<{
    size?: "lg" | "sm";
    state?: AvatarState;
    /** 瞳孔偏移覆盖（画布单位）：提供时优先于内部跟踪（桌宠"张望"动画用） */
    eyeShift?: { x: number; y: number } | null;
    /** 瞳孔跟踪方式：window 窗口内鼠标（默认）/ global 全屏光标轮询（桌宠）/ none 关闭 */
    track?: "window" | "global" | "none";
  }>(),
  { size: "sm", state: "idle", eyeShift: null, track: "window" },
);

/** 状态 → 状态灯/光晕颜色（天线球、working 光环、光晕共用） */
const STATE_COLORS: Record<AvatarState, string> = {
  idle: "var(--accent)",
  thinking: "var(--orange)",
  working: "var(--orange)",
  success: "var(--green)",
  error: "var(--red)",
  dozing: "var(--accent)",
};
const stateColor = computed(() => STATE_COLORS[props.state]);

/* ---------- 瞳孔跟随：eyeShift prop 覆盖 > 内部跟踪（窗口内鼠标 / 全屏光标轮询） ---------- */
const rootEl = ref<HTMLElement | null>(null);
const eyesShift = ref({ x: 0, y: 0 });
const trackStyle = computed(() => {
  const s = props.eyeShift ?? eyesShift.value;
  return {
    transform: `translate(${s.x.toFixed(1)}px, ${s.y.toFixed(1)}px)`,
  };
});
/** 内部跟踪是否启用：大尺寸 + 待机/打盹态 + 未被 eyeShift 覆盖 */
const tracking = computed(
  () => !props.eyeShift && props.size === "lg" && (props.state === "idle" || props.state === "dozing") && props.track !== "none",
);
const reducedMotion =
  typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

/** 方向向量 → 瞳孔偏移（画布单位；距离越远越明显，封顶 1） */
function shiftToward(dx: number, dy: number, reachDivisor: number) {
  const dist = Math.hypot(dx, dy) || 1;
  const reach = Math.min(1, dist / reachDivisor);
  eyesShift.value = {
    x: (dx / dist) * 6.5 * reach,
    y: (dy / dist) * 7 * reach,
  };
}

function onPointerMove(e: MouseEvent) {
  const host = rootEl.value;
  if (!host) return;
  const r = host.getBoundingClientRect();
  const dx = e.clientX - (r.left + r.width / 2);
  const dy = e.clientY - (r.top + r.height * 0.47); // 眼睛约在头部 47% 高度处
  shiftToward(dx, dy, 150); // 近距离衰减，避免在脸上打转时抖动
}

/** 全屏光标轮询（桌宠窗口很小，鼠标常在窗外）：自身窗口中心 vs 系统光标位置 */
let globalTrackTimer = 0;
async function pollGlobalCursor() {
  const host = rootEl.value;
  if (!host) return;
  try {
    const { cursorPosition, getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const [cursor, pos, size] = await Promise.all([
      cursorPosition(),
      win.outerPosition(),
      win.outerSize(),
    ]);
    const cx = pos.x + size.width / 2;
    const cy = pos.y + size.height * 0.47;
    shiftToward(cursor.x - cx, cursor.y - cy, 500);
  } catch {
    /* 非 Tauri 环境（单测）忽略 */
  }
}

function resetEyesShift() {
  eyesShift.value = { x: 0, y: 0 };
}

/* ---------- 随机眨眼：2.4~5s 间隔，150ms 眯合 ---------- */
const blinking = ref(false);
let blinkTimer = 0;
let blinkOffTimer = 0;

function scheduleBlink() {
  window.clearTimeout(blinkTimer);
  blinkTimer = window.setTimeout(
    () => {
      blinking.value = true;
      blinkOffTimer = window.setTimeout(() => {
        blinking.value = false;
        scheduleBlink();
      }, 150);
    },
    2400 + Math.random() * 2600,
  );
}

onMounted(() => {
  if (reducedMotion) return;
  scheduleBlink();
});

watch(
  tracking,
  (on) => {
    if (on) {
      if (props.track === "global") {
        // 桌宠：轮询全屏光标（窗口自身收不到窗外 mousemove）
        window.clearInterval(globalTrackTimer);
        globalTrackTimer = window.setInterval(pollGlobalCursor, 160);
      } else {
        window.addEventListener("mousemove", onPointerMove, { passive: true });
        document.addEventListener("mouseleave", resetEyesShift);
      }
    } else {
      window.clearInterval(globalTrackTimer);
      window.removeEventListener("mousemove", onPointerMove);
      document.removeEventListener("mouseleave", resetEyesShift);
      resetEyesShift();
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  window.clearTimeout(blinkTimer);
  window.clearTimeout(blinkOffTimer);
  window.clearInterval(globalTrackTimer);
  window.removeEventListener("mousemove", onPointerMove);
  document.removeEventListener("mouseleave", resetEyesShift);
});
</script>

<style scoped>
.avatar {
  position: relative;
  flex-shrink: 0;
}
.avatar.size-lg {
  width: 132px;
  height: 132px;
  margin: 0 auto;
}
.avatar.size-sm {
  width: 36px;
  height: 36px;
}

/* 光晕：状态色呼吸（仅大尺寸） */
.halo {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.halo::before {
  content: "";
  position: absolute;
  left: 50%;
  top: 50%;
  width: 72%;
  height: 72%;
  border-radius: 50%;
  background: var(--state-color);
  transform: translate(-50%, -50%);
  animation: halo-pulse 4s ease-in-out infinite;
}

/* 底部投影：与光晕同周期呼吸，营造悬浮感 */
.shadow {
  position: absolute;
  left: 50%;
  bottom: 2px;
  width: 47%;
  height: 7%;
  border-radius: 50%;
  background: var(--state-color);
  transform: translateX(-50%);
  animation: shadow-pulse 4s ease-in-out infinite;
  pointer-events: none;
}

/* 头部浮动；成功轻弹一下、出错蔫蔫下沉 */
.bob {
  position: absolute;
  inset: 0;
  animation: avatar-bob 4s ease-in-out infinite;
}
.bob.hop {
  animation: avatar-hop 0.9s ease-out;
}
.bob.droop {
  animation: avatar-droop 0.9s ease-out;
}

.face {
  width: 100%;
  height: 100%;
  display: block;
  overflow: visible;
}

/* 外壳与屏幕 */
.shell {
  fill: var(--bg-panel);
  stroke: var(--border-strong);
  stroke-width: 4;
}
.screen {
  fill: var(--bg-input);
  stroke: var(--border-soft);
  stroke-width: 3;
}

/* 天线 */
.stem {
  stroke: var(--border-strong);
  stroke-width: 5;
  stroke-linecap: round;
}
.ball {
  fill: var(--state-color);
  transition: fill 0.3s;
  animation: ball-pulse 2.4s ease-in-out infinite;
}
.state-error .ball {
  animation: ball-pulse 1.1s ease-in-out infinite;
}
.state-success .ball {
  animation: none;
}

/* working：天线球外的旋转光环 */
.ring-wrap {
  transform-box: fill-box;
  transform-origin: center;
  animation: ring-spin 1.2s linear infinite;
}
.ring {
  fill: none;
  stroke: var(--state-color);
  stroke-width: 4;
  stroke-linecap: round;
  stroke-dasharray: 24 62;
}

/* thinking：头顶三颗思考点，依次弹跳 */
.think-dots {
  fill: var(--state-color);
}
.think-dots circle {
  animation: dot-bounce 1.2s ease-in-out infinite;
}
.think-dots circle:nth-child(2) {
  animation-delay: 0.15s;
}
.think-dots circle:nth-child(3) {
  animation-delay: 0.3s;
}

/* dozing：天线灯暗淡，头顶 Zzz 依次上浮淡出 */
.state-dozing .ball {
  opacity: 0.35;
  animation: none;
}
.zzz-t {
  fill: var(--text-muted);
  font-family: inherit;
  font-weight: 600;
  animation: zzz-float 2.6s ease-in-out infinite;
}
.zzz-t.s {
  font-size: 13px;
  animation-delay: 0s;
}
.zzz-t.m {
  font-size: 16px;
  animation-delay: 0.5s;
}
.zzz-t.l {
  font-size: 20px;
  animation-delay: 1s;
}
@keyframes zzz-float {
  0% {
    opacity: 0;
    transform: translateY(4px);
  }
  30%,
  70% {
    opacity: 0.9;
  }
  100% {
    opacity: 0;
    transform: translateY(-6px);
  }
}

/* 眼睛 */
.eyes-track {
  transition: transform 0.09s ease-out;
  will-change: transform;
}
.eyes {
  transition: transform 0.25s ease-out;
}
.eyes.look-down {
  transform: translateY(6px);
}
.eye {
  fill: var(--text-main);
  transform-box: fill-box;
  transform-origin: center;
  transition: transform 0.06s ease-out;
}
.eyes.blink .eye {
  transform: scaleY(0.12);
}
.eye-stroke {
  fill: none;
  stroke: var(--text-main);
  stroke-width: 8;
  stroke-linecap: round;
}
.x-eye line {
  stroke: var(--text-main);
  stroke-width: 7;
  stroke-linecap: round;
}

/* 出错歪头 */
.head.tilt {
  transform-box: fill-box;
  transform-origin: center;
  transform: rotate(2.5deg);
  transition: transform 0.3s ease-out;
}

@keyframes avatar-bob {
  0%,
  100% {
    transform: translateY(3px);
  }
  50% {
    transform: translateY(-3px);
  }
}
@keyframes avatar-hop {
  0% {
    transform: translateY(0) scale(1);
  }
  35% {
    transform: translateY(-10px) scale(1.04);
  }
  70% {
    transform: translateY(2px) scale(0.99);
  }
  100% {
    transform: translateY(0) scale(1);
  }
}
@keyframes avatar-droop {
  0% {
    transform: translateY(0);
  }
  40% {
    transform: translateY(6px);
  }
  100% {
    transform: translateY(3px);
  }
}
@keyframes halo-pulse {
  0%,
  100% {
    opacity: 0.07;
    transform: translate(-50%, -50%) scale(0.94);
  }
  50% {
    opacity: 0.13;
    transform: translate(-50%, -50%) scale(1.06);
  }
}
@keyframes shadow-pulse {
  0%,
  100% {
    opacity: 0.14;
  }
  50% {
    opacity: 0.07;
  }
}
@keyframes ball-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.55;
  }
}
@keyframes ring-spin {
  to {
    transform: rotate(360deg);
  }
}
@keyframes dot-bounce {
  0%,
  100% {
    transform: translateY(0);
    opacity: 0.5;
  }
  50% {
    transform: translateY(-5px);
    opacity: 1;
  }
}

/* 减少动态偏好：停掉所有循环/过渡动画，保留静态表情 */
@media (prefers-reduced-motion: reduce) {
  .bob,
  .halo::before,
  .shadow,
  .ball,
  .ring-wrap,
  .think-dots circle,
  .zzz-t,
  .eyes,
  .eye {
    animation: none !important;
    transition: none !important;
  }
}
</style>
