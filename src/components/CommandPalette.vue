<template>
  <Teleport to="body">
    <div v-if="open" class="palette-mask" @click.self="close">
      <div class="palette">
        <div class="palette-input-row">
          <NIcon :component="SearchOutline" :size="18" color="#999" />
          <input
            ref="inputRef"
            v-model="query"
            class="palette-input"
            :placeholder="t('palette.placeholder')"
            @keydown="onKeydown"
          />
          <span class="palette-esc">ESC</span>
        </div>
        <div class="palette-list">
          <div
            v-for="(item, idx) in matches"
            :key="item.id"
            class="palette-item"
            :class="{ active: idx === index }"
            @mouseenter="index = idx"
            @click="choose(idx)"
          >
            <NIcon :component="item.icon" :size="17" :color="item.color" />
            <span class="palette-label">{{ t(item.label) }}</span>
            <span v-if="item.id === active" class="palette-current">{{ t("palette.current") }}</span>
          </div>
          <div v-if="matches.length === 0" class="palette-empty">{{ t("palette.noResult") }}</div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { NIcon } from "naive-ui";
import { useI18n } from "vue-i18n";
import { SearchOutline } from "@vicons/ionicons5";
import { filterNavItems } from "../navItems";
import type { NavId } from "../types";

const props = defineProps<{ active: NavId }>();
const emit = defineEmits<{ (e: "select", id: NavId): void }>();

const { t } = useI18n();

const open = ref(false);
const query = ref("");
const index = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);

/** 按 i18n 名称 / id 模糊过滤 */
const matches = computed(() => filterNavItems(query.value, (k) => t(k)));

function openPalette() {
  query.value = "";
  index.value = 0;
  open.value = true;
  void nextTick(() => inputRef.value?.focus());
}

function close() {
  open.value = false;
}

function choose(idx: number) {
  const item = matches.value[idx];
  if (!item) return;
  emit("select", item.id);
  close();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    index.value = matches.value.length > 0 ? (index.value + 1) % matches.value.length : 0;
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    index.value =
      matches.value.length > 0 ? (index.value - 1 + matches.value.length) % matches.value.length : 0;
  } else if (e.key === "Enter") {
    e.preventDefault();
    choose(index.value);
  } else if (e.key === "Escape") {
    e.preventDefault();
    close();
  }
}

/** 全局快捷键 Cmd/Ctrl+K 唤起 */
function onWindowKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && (e.key === "k" || e.key === "K")) {
    e.preventDefault();
    if (open.value) {
      close();
    } else {
      openPalette();
    }
  }
}

watch(query, () => {
  index.value = 0;
});

onMounted(() => window.addEventListener("keydown", onWindowKeydown));
onUnmounted(() => window.removeEventListener("keydown", onWindowKeydown));
</script>

<style scoped>
.palette-mask { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.35); z-index: 1000; display: flex; justify-content: center; padding-top: 14vh; }
.palette { width: 520px; max-width: 90vw; height: fit-content; max-height: 60vh; background: var(--bg-panel); border-radius: 12px; box-shadow: 0 8px 30px rgba(0, 0, 0, 0.25); display: flex; flex-direction: column; overflow: hidden; }
.palette-input-row { display: flex; align-items: center; gap: 10px; padding: 14px 16px; border-bottom: 1px solid var(--border-soft); }
.palette-input { flex: 1; border: none; outline: none; background: transparent; font-size: 15px; color: var(--text-main); }
.palette-esc { font-size: 11px; color: var(--text-faint); border: 1px solid var(--border-soft); border-radius: 4px; padding: 1px 6px; }
.palette-list { overflow-y: auto; padding: 6px; }
.palette-item { display: flex; align-items: center; gap: 10px; padding: 9px 12px; border-radius: 8px; cursor: pointer; font-size: 14px; color: var(--text-sub); }
.palette-item.active { background: var(--accent-soft); color: var(--text-main); }
.palette-label { flex: 1; }
.palette-current { font-size: 11px; color: var(--accent); }
.palette-empty { padding: 20px; text-align: center; font-size: 13px; color: var(--text-faint); }
</style>
