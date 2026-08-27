<template>
  <div class="pet">
    <!-- EP02 先放个静态机器人占位，EP03 会让它活起来 -->
    <svg viewBox="0 0 120 120" class="robot">
      <!-- 天线 -->
      <line x1="60" y1="10" x2="60" y2="24" stroke="#8a8f98" stroke-width="4" stroke-linecap="round" />
      <circle cx="60" cy="9" r="5" fill="#2080f0" />
      <!-- 头 -->
      <rect x="22" y="24" width="76" height="56" rx="18" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
      <!-- 屏幕脸 -->
      <rect x="30" y="32" width="60" height="40" rx="10" fill="#f2f4f7" stroke="#e3e5e8" stroke-width="2" />
      <!-- 眼睛 -->
      <ellipse cx="48" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
      <ellipse cx="72" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
      <!-- 嘴 -->
      <path d="M 52 63 Q 60 69 68 63" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
      <!-- 身体 -->
      <rect x="40" y="84" width="40" height="18" rx="9" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
    </svg>
  </div>
</template>

<script setup lang="ts">
// EP02：纯展示。透明背景是关键——窗口透明后，页面背景必须也是透明的，
// 否则会看到一块白色矩形。
document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";
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
  /* 轻微的悬浮感，先意思一下 */
  animation: bob 3.6s ease-in-out infinite;
}
@keyframes bob {
  0%, 100% { transform: translateY(2px); }
  50% { transform: translateY(-2px); }
}
</style>
