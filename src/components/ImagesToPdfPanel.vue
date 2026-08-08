<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("images2pdf.title") }}</h2>
      <p>{{ t("images2pdf.subtitle") }}</p>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFiles">
      <div v-if="!images.length" class="zone-empty">
        <NIcon :component="ImagesOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("images2pdf.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="ImagesOutline" :size="20" color="#e6494c" />
        <span>{{ t("images2pdf.added", { n: images.length }) }}</span>
      </div>
    </div>

    <!-- 图片列表 -->
    <div v-if="images.length" class="file-list">
      <div v-for="(f, i) in images" :key="f" class="file-row">
        <span class="order">{{ i + 1 }}</span>
        <NIcon :component="ImageOutline" :size="19" color="#e6494c" />
        <span class="fname" :title="f">{{ names[i] }}</span>
        <button class="remove-btn" @click="images.splice(i, 1)">×</button>
      </div>
    </div>

    <!-- 页面尺寸配置 -->
    <div v-if="images.length" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("images2pdf.sizeLabel") }}</label>
        <NRadioGroup v-model:value="pageSize">
          <div class="size-options">
            <NRadioButton value="auto" :label="t('images2pdf.sizeAuto')" />
            <NRadioButton value="a4" :label="t('images2pdf.sizeA4')" />
          </div>
        </NRadioGroup>
        <span class="config-hint">
          {{ pageSize === "auto" ? t("images2pdf.sizeAutoHint") : t("images2pdf.sizeA4Hint") }}
        </span>
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("images2pdf.hint") }}</span>
      <button class="cta" :disabled="!images.length" @click="doConvert">
        <NIcon :component="ImagesOutline" :size="17" />
        {{ t("images2pdf.cta") }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NRadioButton, NRadioGroup, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ImageOutline, ImagesOutline } from "@vicons/ionicons5";
import { imagesToPdf } from "../api";
import { useHistoryStore } from "../stores/history";

const { t } = useI18n();
const message = useMessage();
const history = useHistoryStore();

/** 支持图片扩展名（与后端 image crate 支持对齐） */
const IMAGE_EXTS = ["png", "jpg", "jpeg", "bmp", "gif", "webp"];

const images = ref<string[]>([]);
const names = computed(() => images.value.map((p) => p.split(/[\\/]/).pop() ?? p));
/** 页面尺寸：跟随图片 / A4 居中 */
const pageSize = ref<"auto" | "a4">("auto");

async function pickFiles() {
  const paths = await openDialog({
    multiple: true,
    filters: [{ name: t("common.docs"), extensions: IMAGE_EXTS }],
  });
  if (!paths) return;
  for (const p of paths) {
    const path = String(p);
    if (!images.value.includes(path)) images.value.push(path);
  }
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
function handleDrop(paths: string[]) {
  const imgs = paths.filter((p) => IMAGE_EXTS.some((e) => p.toLowerCase().endsWith(`.${e}`)));
  if (!imgs.length) {
    message.warning(t("images2pdf.warnNoImage"));
    return;
  }
  for (const p of imgs) {
    if (!images.value.includes(p)) images.value.push(p);
  }
}
defineExpose({ handleDrop });

async function doConvert() {
  if (!images.value.length) {
    message.warning(t("images2pdf.warnNoFile"));
    return;
  }
  const defaultName = `images_${Date.now().toString().slice(-6)}.pdf`;
  const outPath = await openDialog({
    save: true,
    title: t("images2pdf.saveTitle"),
    defaultPath: defaultName,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!outPath) return;
  try {
    const out = await imagesToPdf([...images.value], String(outPath), pageSize.value);
    const outName = out.split(/[\\/]/).pop() ?? out;
    message.success(t("images2pdf.success", { name: outName }), { duration: 4000 });
    await history.add({
      kind: "images2pdf",
      name: outName,
      inputs: [...images.value],
      outputs: [out],
      ok: true,
    });
  } catch (e) {
    message.error(t("images2pdf.fail", { err: String(e) }));
    await history.add({
      kind: "images2pdf",
      name: `${names.value[0]}${names.value.length > 1 ? ` ${t("common.etc")}${names.value.length}` : ""}`,
      inputs: [...images.value],
      outputs: [],
      ok: false,
    });
  }
}
</script>

<style scoped>
.panel {
  background: var(--bg-panel);
  border-radius: 14px;
  padding: 30px;
  box-shadow: 0 1px 3px var(--shadow);
}
.panel-head h2 {
  margin: 0;
  font-size: 22px;
  color: var(--text-main);
}
.panel-head p {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-muted);
}
.upload-zone {
  margin-top: 18px;
  border: 2px dashed var(--border-dash);
  border-radius: 14px;
  padding: 28px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
}
.upload-zone:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.zone-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}
.zone-main {
  margin: 0;
  font-size: 14px;
  color: var(--text-sub);
}
.zone-main .link {
  color: var(--accent);
  cursor: pointer;
}
.zone-sub {
  margin: 0;
  font-size: 12px;
  color: var(--text-faint);
}
.zone-filled {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 14px;
  color: var(--text-sub);
}
.file-list {
  margin-top: 14px;
}
.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 6px;
  border-top: 1px solid var(--border-soft);
}
.order {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--bg-tag);
  color: var(--text-muted);
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.fname {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--text-body);
}
.remove-btn {
  border: none;
  background: none;
  color: var(--text-muted);
  font-size: 18px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}
.remove-btn:hover { color: var(--red); }
.config {
  margin-top: 18px;
}
.config-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.config-label {
  font-size: 13px;
  color: var(--text-sub);
}
.size-options {
  display: flex;
  gap: 8px;
}
.config-hint {
  font-size: 12px;
  color: var(--text-faint);
}
.action-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 18px;
  padding-top: 16px;
  border-top: 1px solid var(--border-soft);
}
.hint {
  font-size: 12px;
  color: var(--text-muted);
}
.cta {
  display: flex;
  align-items: center;
  gap: 8px;
  border: none;
  background: var(--cta-bg);
  color: var(--cta-text);
  font-size: 15px;
  font-weight: 600;
  padding: 11px 30px;
  border-radius: 8px;
  cursor: pointer;
  transition: opacity 0.15s;
}
.cta:hover:not(:disabled) { opacity: 0.85; }
.cta:disabled {
  background: var(--cta-disabled);
  cursor: not-allowed;
}
</style>
