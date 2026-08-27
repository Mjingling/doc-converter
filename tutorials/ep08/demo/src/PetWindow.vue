<template>
  <div class="pet">
    <div v-if="bubbleText" class="bubble">
      <span>{{ bubbleText }}</span>
      <div v-if="mood === 'working'" class="mini-bar">
        <div class="mini-fill" :style="{ width: progress + '%' }"></div>
      </div>
    </div>

    <svg viewBox="0 0 120 120" class="robot" :class="{ celebrate: mood === 'happy' }">
      <line x1="60" y1="10" x2="60" y2="24" stroke="#8a8f98" stroke-width="4" stroke-linecap="round" />
      <circle cx="60" cy="9" r="5" :fill="mood === 'working' ? '#f0a020' : '#2080f0'" />
      <rect x="22" y="24" width="76" height="56" rx="18" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
      <rect x="30" y="32" width="60" height="40" rx="10" fill="#f2f4f7" stroke="#e3e5e8" stroke-width="2" />
      <template v-if="mood === 'happy'">
        <path d="M 42 54 Q 48 46 54 54" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
        <path d="M 66 54 Q 72 46 78 54" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
      </template>
      <template v-else>
        <ellipse cx="48" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
        <ellipse cx="72" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
      </template>
      <path d="M 52 63 Q 60 69 68 63" fill="none" stroke="#1a1a1a" stroke-width="3" stroke-linecap="round" />
      <rect x="40" y="84" width="40" height="18" rx="9" fill="#ffffff" stroke="#c9ccd1" stroke-width="3" />
    </svg>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { PET_PROGRESS_EVENT, type PetProgressPayload } from "./petProgress";

const { t } = useI18n();

document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";

type Mood = "idle" | "working" | "happy" | "sad";
const mood = ref<Mood>("idle");
const progress = ref(0);
const bubbleText = ref("");
let bubbleTimer = 0;

function say(text: string, ms = 2600) {
  bubbleText.value = text;
  clearTimeout(bubbleTimer);
  bubbleTimer = window.setTimeout(() => (bubbleText.value = ""), ms);
}

let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  unlisten = await listen<PetProgressPayload>(PET_PROGRESS_EVENT, (e) => {
    const p = e.payload;
    switch (p.phase) {
      case "start":
        mood.value = "working";
        progress.value = 0;
        say(t("pet.start"), 60000);
        break;
      case "tick":
        mood.value = "working";
        progress.value = p.progress ?? 0;
        bubbleText.value = t("pet.working", { pct: Math.round(progress.value) });
        break;
      case "done":
        mood.value = "happy";
        say(t("pet.done", { name: p.name ?? "" }));
        window.setTimeout(() => (mood.value = "idle"), 3000);
        break;
      case "error":
        mood.value = "sad";
        say(t("pet.error"), 3500);
        window.setTimeout(() => (mood.value = "idle"), 3500);
        break;
    }
  });
});

onUnmounted(() => {
  unlisten?.();
  clearTimeout(bubbleTimer);
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
.robot.celebrate { animation: hop 0.6s ease-out 2; }
.bubble {
  position: absolute;
  top: 4px;
  left: 50%;
  transform: translateX(-50%);
  min-width: 96px;
  max-width: 132px;
  padding: 5px 10px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #e3e5e8;
  font-size: 12px;
  text-align: center;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}
.mini-bar {
  margin-top: 4px;
  height: 5px;
  border-radius: 3px;
  background: #eef0f3;
  overflow: hidden;
}
.mini-fill { height: 100%; background: #2080f0; transition: width 0.15s linear; }
@keyframes bob {
  0%, 100% { transform: translateY(2px); }
  50% { transform: translateY(-2px); }
}
@keyframes hop {
  0% { transform: translateY(0); }
  40% { transform: translateY(-16px); }
  100% { transform: translateY(0); }
}
</style>
