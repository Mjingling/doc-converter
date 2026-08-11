<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("outline.title") }}</h2>
      <p>{{ t("outline.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile" @dragover.prevent @drop.prevent="onDrop">
      <div v-if="!filePath" class="zone-empty">
        <NIcon :component="BookmarkOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("outline.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="filePath">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
        <button class="clear-btn" @click.stop="clearFile">&times;</button>
      </div>
    </div>

    <div v-if="filePath" class="form">
      <div class="items-header">
        <label>{{ t("outline.itemLabel") }}</label>
        <NButton size="tiny" @click="addItem">{{ t("outline.addItem") }}</NButton>
      </div>
      <div v-for="(item, idx) in items" :key="idx" class="item-row">
        <NInput v-model:value="item.title" :placeholder="t('outline.titlePlaceholder')" size="small" />
        <NInputNumber v-model:value="item.page" :min="1" :placeholder="t('outline.pagePlaceholder')" size="small" style="width:100px" />
        <button class="remove-btn" @click="items.splice(idx, 1)">{{ t("outline.removeItem") }}</button>
      </div>
    </div>

    <!-- 输出目录 -->
    <div v-if="filePath" class="out-dir" @click="pickDir">
      <span class="out-dir-label">{{ t("outline.outDirLabel") }}</span>
      <span class="out-dir-text">{{ outDir || t("outline.outDirPlaceholder") }}</span>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("outline.hint") }}</span>
      <button class="cta" :disabled="!filePath || items.length === 0" @click="run">
        <NIcon :component="BookmarkOutline" :size="17" />
        {{ t("outline.cta") }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage, NIcon, NInput, NInputNumber, NButton } from "naive-ui";
import { useI18n } from "vue-i18n";
import { BookmarkOutline, DocumentTextOutline } from "@vicons/ionicons5";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { pdfOutline } from "../api";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";
import { dirOf } from "../utils/file";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();
const settings = useSettingsStore();

const filePath = ref("");
const fileName = ref("");
const items = ref<{ title: string; page: number }[]>([]);
// 输出目录：初始化用设置中的默认目录；手动选择后记住上次目录
const outDir = ref(settings.defaultOutDir);
let lastChosenDir = "";

async function pickDir() {
  const d = await openDialog({ directory: true, title: t("outline.pickDirTitle") });
  if (d) {
    outDir.value = String(d);
    lastChosenDir = String(d);
  }
}

function handleFile(path: string) {
  if (!path.toLowerCase().endsWith(".pdf")) {
    message.warning(t("outline.warnOnlyPdf"));
    return;
  }
  filePath.value = path;
  fileName.value = path.split(/[/\\]/).pop() || path;
}

function clearFile() {
  filePath.value = "";
  fileName.value = "";
  items.value = [];
}

function addItem() {
  items.value.push({ title: "", page: 1 });
}

async function pickFile() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const sel = await open({ filters: [{ name: "PDF", extensions: ["pdf"] }], multiple: false });
  if (sel) handleFile(sel);
}

function onDrop(e: DragEvent) {
  const f = (e.dataTransfer?.files)?.[0];
  if (f) handleFile((f as any).path);
}

async function run() {
  if (!filePath.value || items.value.length === 0) return;
  // 输出目录：上次手动选择的目录优先；没有则输出到源文件所在目录
  let dir = outDir.value || lastChosenDir;
  if (!dir) dir = dirOf(filePath.value);
  const stem = fileName.value.replace(/\.pdf$/i, "");
  const out = `${dir}/${stem}_bookmarked.pdf`;
  try {
    const data: [string, number][] = items.value.map(i => [i.title || `Bookmark ${i.page}`, i.page || 1]);
    await pdfOutline(filePath.value, out, data);
    history.add({ kind: "outline", name: fileName.value, inputs: [filePath.value], outputs: [out], ok: true });
    message.success(t("outline.success", { name: fileName.value }));
  } catch (e: any) {
    history.add({ kind: "outline", name: fileName.value, inputs: [filePath.value], outputs: [], ok: false });
    message.error(t("outline.fail", { err: e }));
  }
}

defineExpose({ handleDrop: handleFile });
</script>

<style scoped>
.panel { background: var(--bg-panel); border-radius: 14px; padding: 30px; box-shadow: 0 1px 3px var(--shadow); }
.panel-head h2 { margin: 0; font-size: 22px; color: var(--text-main); }
.panel-head p { margin: 6px 0 0; font-size: 13px; color: var(--text-muted); }
.upload-zone { margin-top: 18px; border: 2px dashed var(--border-dash); border-radius: 14px; padding: 28px; text-align: center; cursor: pointer; transition: all 0.2s; }
.upload-zone:hover { border-color: var(--accent); background: var(--accent-soft); }
.zone-empty { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.zone-main { margin: 0; font-size: 14px; color: var(--text-sub); }
.zone-main .link { color: var(--accent); cursor: pointer; }
.zone-sub { margin: 0; font-size: 12px; color: var(--text-faint); }
.zone-filled { display: flex; align-items: center; justify-content: center; gap: 10px; font-size: 14px; color: var(--text-sub); }
.fname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: var(--text-body); }
.size-tag { font-size: 11px; padding: 2px 8px; border-radius: 8px; color: var(--text-muted); background: var(--bg-tag); }
.clear-btn { border: none; background: none; color: var(--text-muted); font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.clear-btn:hover { color: var(--red); }
.form { margin-top: 18px; display: flex; flex-direction: column; gap: 14px; }
.items-header { display: flex; align-items: center; justify-content: space-between; }
.items-header label { font-size: 13px; color: var(--text-sub); }
.item-row { display: flex; align-items: center; gap: 8px; }
.item-row :first-child { flex: 1; }
.remove-btn { border: none; background: none; color: var(--red); font-size: 12px; cursor: pointer; white-space: nowrap; padding: 0; }
.hint { font-size: 12px; color: var(--text-muted); margin: 0; }
/* 输出目录选择器 */
.out-dir {
  margin-top: 18px;
  padding: 10px 14px;
  border: 1px dashed var(--border-dash);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.out-dir:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.out-dir-label {
  font-size: 13px;
  color: var(--text-sub);
}
.out-dir-text {
  font-size: 13px;
  color: var(--text-muted);
}
.action-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border-soft); }
.cta { display: flex; align-items: center; gap: 8px; border: none; background: var(--cta-bg); color: var(--cta-text); font-size: 15px; font-weight: 600; padding: 11px 30px; border-radius: 8px; cursor: pointer; transition: opacity 0.15s; }
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled { background: var(--cta-disabled); cursor: not-allowed; }
</style>