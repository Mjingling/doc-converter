<template>
  <Teleport to="body">
    <div v-if="visible" class="qd-mask" @click.self="close">
      <div class="qd-modal">
        <div class="qd-head">
          <h3>{{ t("quickDrop.title") }}</h3>
          <button class="qd-close" @click="close">&times;</button>
        </div>

        <!-- 文件列表 -->
        <div class="qd-files">
          <div v-for="p in paths" :key="p" class="qd-file" :title="p">
            <NIcon :component="DocumentTextOutline" :size="16" color="#2080f0" />
            <span class="qd-fname">{{ p.split(/[/\\]/).pop() }}</span>
          </div>
        </div>

        <div v-if="running" class="qd-running">
          <span class="qd-spinner"></span>
          {{ t("quickDrop.running") }}
        </div>

        <template v-else>
          <!-- 无可用操作 -->
          <p v-if="actions.length === 0" class="qd-empty">{{ t("quickDrop.noAction") }}</p>

          <!-- 一键转换：目标格式列表 -->
          <div v-if="actions.includes('convert')" class="qd-section">
            <p class="qd-section-title">{{ t("quickDrop.convertTitle") }}</p>
            <div v-if="formatsLoading" class="qd-empty">{{ t("quickDrop.running") }}</div>
            <div v-else-if="formats.length === 0" class="qd-empty">{{ t("quickDrop.noFormats") }}</div>
            <div v-else class="qd-formats">
              <button
                v-for="f in formats"
                :key="f.ext"
                class="qd-format"
                @click="runConvert(f.ext)"
              >
                {{ f.label }}
              </button>
            </div>
          </div>

          <!-- 其他快捷操作 -->
          <div v-if="otherActions.length > 0" class="qd-section">
            <p class="qd-section-title">{{ t("quickDrop.otherTitle") }}</p>
            <div class="qd-actions">
              <button v-if="actions.includes('compress')" class="qd-action" @click="runCompress">
                <NIcon :component="ArchiveOutline" :size="17" />
                {{ t("quickDrop.compress") }}
              </button>
              <button v-if="actions.includes('images2pdf')" class="qd-action" @click="runImagesToPdf">
                <NIcon :component="ImagesOutline" :size="17" />
                {{ t("quickDrop.images2pdf") }}
              </button>
              <button v-if="actions.includes('aiSummary')" class="qd-action" @click="gotoSummary">
                <NIcon :component="SparklesOutline" :size="17" />
                {{ t("quickDrop.aiSummary") }}
              </button>
            </div>
          </div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NIcon, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { ArchiveOutline, DocumentTextOutline, ImagesOutline, SparklesOutline } from "@vicons/ionicons5";
import { convertDocument, getTargetFormats, imagesToPdf, pdfCompress } from "../api";
import { defaultOutputPath, defaultOutDir } from "../utils/file";
import { parseQuickDropActions, type QuickDropAction } from "../utils/quickDrop";
import { useSettingsStore } from "../stores/settings";
import { useEngineStore } from "../stores/engine";
import { useHistoryStore } from "../stores/history";
import { notifyDone } from "../utils/notify";
import { maybeAutoOpenOutput } from "../utils/autoOpen";
import type { FormatInfo } from "../types";

const props = defineProps<{ visible: boolean; paths: string[] }>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "goto-summary", paths: string[]): void;
}>();

const { t } = useI18n();
const message = useMessage();
const settings = useSettingsStore();
const engine = useEngineStore();
const history = useHistoryStore();

const actions = ref<QuickDropAction[]>([]);
const formats = ref<FormatInfo[]>([]);
const formatsLoading = ref(false);
const running = ref(false);

/** 除转换外的其他操作（用于分组标题展示） */
const otherActions = computed(() => actions.value.filter((a) => a !== "convert"));

/** 主文件：第一个路径（决定可用操作） */
const first = computed(() => props.paths[0] ?? "");

watch(
  () => [props.visible, props.paths],
  async ([visible]) => {
    if (!visible || props.paths.length === 0) return;
    actions.value = parseQuickDropActions(first.value);
    formats.value = [];
    formatsLoading.value = false;
    if (actions.value.includes("convert")) {
      formatsLoading.value = true;
      try {
        formats.value = await getTargetFormats(first.value, engine.mode);
      } catch {
        formats.value = [];
      } finally {
        formatsLoading.value = false;
      }
    }
  },
  { immediate: true }
);

function close() {
  emit("close");
}

async function finish(kind: string, name: string, inputs: string[], out: string) {
  await history.add({ kind, name, inputs, outputs: [out], ok: true });
  message.success(t("quickDrop.done", { name: out.split(/[/\\]/).pop() }));
  void notifyDone(t("common.taskDone"), out.split(/[/\\]/).pop() || "");
  void maybeAutoOpenOutput(out);
  close();
}

async function runConvert(targetExt: string) {
  if (running.value) return;
  running.value = true;
  try {
    const out = await convertDocument(
      first.value,
      targetExt,
      defaultOutDir(first.value, settings.defaultOutDir),
      engine.mode
    );
    await finish("quickDrop", first.value.split(/[/\\]/).pop() || first.value, props.paths, out);
  } catch (e: any) {
    message.error(t("quickDrop.fail", { err: String(e) }));
  } finally {
    running.value = false;
  }
}

async function runCompress() {
  if (running.value) return;
  running.value = true;
  try {
    const out = await pdfCompress(
      first.value,
      defaultOutputPath(first.value, "_compressed", settings.defaultOutDir)
    );
    await finish("quickDrop", first.value.split(/[/\\]/).pop() || first.value, props.paths, out);
  } catch (e: any) {
    message.error(t("quickDrop.fail", { err: String(e) }));
  } finally {
    running.value = false;
  }
}

async function runImagesToPdf() {
  if (running.value) return;
  running.value = true;
  try {
    // 多图全部合成；命名跟随第一张图
    const out = await imagesToPdf(
      props.paths,
      defaultOutputPath(first.value, "_to_pdf", settings.defaultOutDir, ".pdf"),
      "auto"
    );
    await finish("quickDrop", first.value.split(/[/\\]/).pop() || first.value, props.paths, out);
  } catch (e: any) {
    message.error(t("quickDrop.fail", { err: String(e) }));
  } finally {
    running.value = false;
  }
}

function gotoSummary() {
  emit("goto-summary", props.paths);
  close();
}
</script>

<style scoped>
.qd-mask { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.35); z-index: 1000; display: flex; align-items: center; justify-content: center; }
.qd-modal { width: 480px; max-width: 90vw; max-height: 70vh; overflow-y: auto; background: var(--bg-panel); border-radius: 14px; box-shadow: 0 8px 30px rgba(0, 0, 0, 0.25); padding: 22px; }
.qd-head { display: flex; align-items: center; justify-content: space-between; }
.qd-head h3 { margin: 0; font-size: 17px; color: var(--text-main); }
.qd-close { border: none; background: none; font-size: 22px; color: var(--text-muted); cursor: pointer; line-height: 1; }
.qd-close:hover { color: var(--red); }
.qd-files { margin-top: 12px; display: flex; flex-direction: column; gap: 6px; }
.qd-file { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-sub); background: var(--bg-tag); border-radius: 8px; padding: 6px 10px; }
.qd-fname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.qd-running { display: flex; align-items: center; gap: 10px; margin-top: 16px; font-size: 13px; color: var(--text-muted); }
.qd-spinner { width: 14px; height: 14px; border: 2px solid var(--border-soft); border-top-color: var(--accent); border-radius: 50%; animation: qd-spin 0.8s linear infinite; }
@keyframes qd-spin { to { transform: rotate(360deg); } }
.qd-empty { margin: 14px 0 0; font-size: 13px; color: var(--text-faint); }
.qd-section { margin-top: 16px; }
.qd-section-title { margin: 0 0 8px; font-size: 13px; color: var(--text-sub); }
.qd-formats { display: flex; flex-wrap: wrap; gap: 8px; }
.qd-format { border: 1px solid var(--border-soft); background: var(--bg-tag); color: var(--text-sub); font-size: 13px; border-radius: 8px; padding: 7px 14px; cursor: pointer; transition: all 0.15s; }
.qd-format:hover { border-color: var(--accent); color: var(--accent); }
.qd-actions { display: flex; flex-direction: column; gap: 8px; }
.qd-action { display: flex; align-items: center; gap: 10px; border: 1px solid var(--border-soft); background: transparent; color: var(--text-sub); font-size: 13px; border-radius: 8px; padding: 10px 14px; cursor: pointer; transition: all 0.15s; text-align: left; }
.qd-action:hover { border-color: var(--accent); color: var(--accent); }
</style>
