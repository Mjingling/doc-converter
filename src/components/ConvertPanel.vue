<template>
  <div class="panel">
    <!-- 面板头 -->
    <div class="panel-head">
      <h2>{{ t(scene.title) }}</h2>
      <p>{{ t(scene.subtitle) }}</p>
    </div>

    <!-- 引擎提示条：轻量模式提示 / LibreOffice 就绪或缺失提示 -->
    <div v-if="scene.engineRequired" class="engine-tip" :class="tipClass">
      <template v-if="isDynamicScene && engine.mode === 'builtin'">
        <span class="tip-dot light"></span>
        {{ t("convert.lightTip") }}
      </template>
      <template v-else-if="engineReady">
        <span class="tip-dot ok"></span>
        {{ t("convert.tipOk") }}
        <span class="tip-path">{{ engine.path }}</span>
      </template>
      <template v-else>
        <span class="tip-dot warn"></span>
        {{ t("convert.tipWarn") }}
        <button class="tip-btn" @click="switchToLibreOffice">{{ t("convert.goSwitch") }}</button>
        <button class="tip-btn ghost" @click="redetectEngine">{{ t("common.redetect") }}</button>
        <span class="tip-sub">{{ t("convert.subDownload") }}</span>
      </template>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFiles">
      <div v-if="!files.length" class="zone-empty">
        <div class="zone-icons">
          <span class="zone-badge doc">DOC</span>
          <NIcon :component="SwapHorizontalOutline" :size="26" color="#c0c4cc" class="zone-swap" />
          <span class="zone-badge pdf">PDF</span>
        </div>
        <p class="zone-main">
          {{ t("common.uploadBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("convert.zoneSub", { formats: acceptLabel }) }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="CloudUploadOutline" :size="20" color="#2080f0" />
        <span>{{ t("convert.added", { n: files.length }) }}</span>
      </div>
    </div>

    <!-- 文件夹批量添加 -->
    <div class="batch-row">
      <button class="batch-btn" :disabled="converting" @click="pickFolder">
        <NIcon :component="FolderOpenOutline" :size="15" />
        {{ t("convert.pickFolder") }}
      </button>
      <span class="batch-hint">{{ t("convert.pickFolderHint") }}</span>
    </div>

    <!-- 文件列表 -->
    <div v-if="files.length" class="file-list">
      <div v-for="(f, i) in files" :key="f.path" class="file-row">
        <NIcon :component="fmtIcon(f.ext)" :size="19" color="#2080f0" />
        <span class="fname" :title="f.path">{{ f.name }}</span>
        <span class="fext">.{{ f.ext }}</span>
        <span class="arrow">→</span>
        <NSelect
          v-if="f.targets.length"
          :value="f.targetExt"
          :options="f.targets.map((tg) => ({ label: t(tg.label), value: tg.ext }))"
          size="small"
          style="width: 170px"
          @update:value="(v: string) => (f.targetExt = v)"
        />
        <span v-else class="unsupported">
          {{ isDynamicScene && engine.mode === "builtin" ? t("convert.needLo") : t("convert.unsupported") }}
        </span>
        <button class="remove-btn" @click.stop="files.splice(i, 1)">×</button>
      </div>
    </div>

    <!-- 操作区 -->
    <div v-if="files.length" class="action-row">
      <button class="cta" :disabled="!canConvert || converting" @click="startConvert">
        <NIcon :component="CloudUploadOutline" :size="17" />
        {{ ctaLabel }}
      </button>
    </div>

    <!-- 转换进度条 -->
    <div v-if="converting" class="progress-wrap">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
      </div>
      <span class="progress-text">{{ convertedCount }}/{{ convertTotal }} · {{ progressPercent }}%</span>
    </div>

    <!-- 任务结果 -->
    <div v-if="tasks.length" class="task-list">
      <div class="task-title">{{ t("convert.result") }}</div>
      <div v-for="task in tasks" :key="task.id" class="task-row">
        <NIcon :component="task.icon" :size="17" :color="task.color" />
        <span class="fname">{{ task.fileName }}</span>
        <span class="fext">→ {{ task.targetExt }}</span>
        <span class="task-status" :style="{ color: task.color }">{{ t(task.label) }}</span>
        <template v-if="task.outputPath">
          <button class="link-btn" @click="openPath(task.outputPath!)">{{ t("common.open") }}</button>
          <button class="link-btn" @click="openPath(dirOf(task.outputPath!))">
            {{ t("common.openDir") }}
          </button>
        </template>
        <span v-if="task.error" class="task-error" :title="task.error">{{ task.error.slice(0, 50) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NIcon, NSelect, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  CloudUploadOutline, FolderOpenOutline, SwapHorizontalOutline,
  DocumentOutline, GridOutline, EaselOutline, ImageOutline,
  CheckmarkCircleOutline, CloseCircleOutline, SyncOutline,
} from "@vicons/ionicons5";
import { convertDocument, getTargetFormats, openPath, scanDirectory } from "../api";
import { dirOf, defaultOutDir } from "../utils/file";
import { triggerOutputDirPrompt } from "../composables/useOutputDirPrompt";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEngineStore } from "../stores/engine";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";
import type { ConvertScene, ConvertTarget } from "../types";

const { t } = useI18n();
const props = defineProps<{
  /** 场景配置（由 Home.vue 传入） */
  scene: ConvertScene;
}>();

const message = useMessage();
const engine = useEngineStore();
const settings = useSettingsStore();
const history = useHistoryStore();

/** 引擎是否就绪（LibreOffice 模式且已安装） */
const engineReady = computed(() => engine.mode === "libreoffice" && engine.available);

/** 动态格式场景（无固定目标格式，可随引擎切换） */
const isDynamicScene = computed(() => !props.scene.fixedTargets?.length);

/** 提示条样式：轻量模式 / LibreOffice 就绪 / 缺失 */
const tipClass = computed(() => {
  if (isDynamicScene.value && engine.mode === "builtin") return "light";
  return engineReady.value ? "ok" : "warn";
});

/** 可接受扩展名文案，如「PDF」「DOC / DOCX / ODT 等」 */
const acceptLabel = computed(() => {
  const exts = props.scene.acceptExts;
  return exts.length > 3 ? `${exts.slice(0, 3).join(" / ")} ${t("common.etc")}` : exts.join(" / ");
});

/* ---------- 文件列表 ---------- */
interface FileItem {
  path: string;
  name: string;
  ext: string;
  targets: ConvertTarget[];
  targetExt: string;
}
const files = ref<FileItem[]>([]);

/** 文件类型图标 */
const fmtIcons: Record<string, any> = {
  pdf: DocumentOutline, doc: DocumentOutline, docx: DocumentOutline,
  odt: DocumentOutline, rtf: DocumentOutline, txt: DocumentOutline, html: DocumentOutline,
  xls: GridOutline, xlsx: GridOutline, ods: GridOutline, csv: GridOutline,
  ppt: EaselOutline, pptx: EaselOutline, odp: EaselOutline,
  png: ImageOutline, jpg: ImageOutline,
};
function fmtIcon(ext: string): any {
  return fmtIcons[ext] || DocumentOutline;
}

function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}

async function addFile(path: string) {
  const name = path.split(/[\\/]/).pop() || path;
  const ext = extOf(name);
  // 过滤不支持的扩展名
  if (!props.scene.acceptExts.includes(ext)) {
    message.warning(t("convert.unsupportedFile", { name, formats: acceptLabel.value }));
    return;
  }
  if (files.value.some((f) => f.path === path)) {
    message.warning(t("convert.alreadyAdded", { name }));
    return;
  }
  const item: FileItem = { path, name, ext, targets: [], targetExt: "" };
  files.value.push(item);
  triggerOutputDirPrompt(path);
  await loadTargets(item);
}

/** 加载单个文件的目标格式（按当前引擎模式） */
async function loadTargets(item: FileItem) {
  try {
    // 静态目标格式优先，否则从后端动态获取
    if (props.scene.fixedTargets?.length) {
      item.targets = [...props.scene.fixedTargets];
    } else {
      item.targets = await getTargetFormats(item.path, engine.mode);
    }
    item.targetExt = item.targets[0]?.ext ?? "";
  } catch (e) {
    item.targets = [];
    item.targetExt = "";
    message.error(t("convert.identifyFail", { name: item.name, err: String(e) }));
  }
}

/** 引擎切换后重新获取各文件的目标格式 */
watch(
  () => engine.mode,
  () => {
    for (const f of files.value) void loadTargets(f);
  }
);

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
function handleDrop(paths: string[]) {
  for (const p of paths) void addFile(p);
}
defineExpose({ handleDrop });

async function pickFiles() {
  const paths = await openDialog({
    multiple: true,
    filters: [{ name: t("common.docs"), extensions: props.scene.acceptExts }],
  });
  if (paths) for (const p of paths) await addFile(p as string);
}

/** 选择文件夹：递归扫描其中的支持文件并批量加入列表（跳过已添加项） */
async function pickFolder() {
  const dir = await openDialog({ directory: true, title: t("convert.pickFolderTitle") });
  if (!dir) return;
  try {
    const paths = await scanDirectory(String(dir), props.scene.acceptExts);
    if (!paths.length) {
      message.info(t("convert.noFilesInDir"));
      return;
    }
    let added = 0;
    for (const p of paths) {
      const name = p.split(/[\\/]/).pop() || p;
      const ext = extOf(name);
      if (!props.scene.acceptExts.includes(ext)) continue;
      if (files.value.some((f) => f.path === p)) continue;
      files.value.push({ path: p, name, ext, targets: [], targetExt: "" });
      added++;
      void loadTargets(files.value[files.value.length - 1]);
    }
    message.success(t("convert.dirAdded", { n: added }));
  } catch (e) {
    message.error(t("convert.dirScanFail", { err: String(e) }));
  }
}

/* ---------- 转换执行 ---------- */
const readyCount = computed(() => files.value.filter((f) => f.targetExt).length);
// 内置引擎：仅动态场景（轻量转换）可用，固定目标场景（如 PDF→Word）需 LibreOffice
const canConvert = computed(() => {
  if (!readyCount.value) return false;
  if (engine.mode === "builtin") return isDynamicScene.value;
  return engineReady.value;
});

/* 转换中状态：禁用按钮 + 进度条 */
const converting = ref(false);
const convertTotal = ref(0);
const convertedCount = ref(0);
const progressPercent = computed(() =>
  convertTotal.value ? Math.round((convertedCount.value / convertTotal.value) * 100) : 0
);
const ctaLabel = computed(() => {
  if (converting.value) return t("convert.converting", { done: convertedCount.value, total: convertTotal.value });
  if (engine.mode === "builtin") {
    return isDynamicScene.value ? t("convert.start", { n: readyCount.value }) : t("convert.needEngine");
  }
  if (engineReady.value) return t("convert.start", { n: readyCount.value });
  return t("convert.needEngine");
});

interface TaskItem {
  id: string;
  fileName: string;
  targetExt: string;
  /** i18n key：状态文案 */
  label: string;
  color: string;
  icon: any;
  outputPath?: string;
  error?: string;
}
let seq = 0;
const tasks = ref<TaskItem[]>([]);

async function startConvert() {
  const targets = files.value.filter((f) => f.targetExt);
  if (!targets.length) {
    message.warning(t("convert.noTarget"));
    return;
  }
  if (engine.mode === "libreoffice" && !engineReady.value) {
    message.warning(t("convert.noEngine"));
    return;
  }
  const dir = defaultOutDir(targets[0].path, settings.defaultOutDir);
  // 开始转换：禁用按钮并显示进度
  converting.value = true;
  convertTotal.value = targets.length;
  convertedCount.value = 0;
  const outputs: string[] = [];
  let okCount = 0;
  try {
    for (const f of targets) {
      const id = `task-${++seq}-${Date.now()}`;
      tasks.value.unshift({
        id, fileName: f.name, targetExt: f.targetExt,
        label: "common.converting", color: "#2080f0", icon: SyncOutline,
      });
      try {
        const out = await convertDocument(f.path, f.targetExt, dir, engine.mode);
        outputs.push(out);
        okCount++;
        const item = tasks.value.find((x) => x.id === id);
        if (item) Object.assign(item, { label: "common.ok", color: "#18a058", icon: CheckmarkCircleOutline, outputPath: out });
        message.success(t("convert.success", { name: f.name, ext: f.targetExt }));
      } catch (e) {
        const item = tasks.value.find((x) => x.id === id);
        if (item) Object.assign(item, { label: "common.fail", color: "#e6494c", icon: CloseCircleOutline, error: String(e) });
        message.error(t("convert.fail", { name: f.name, err: String(e) }));
      }
      convertedCount.value++;
    }
    // 全部完成：写入历史（汇总一条），窗口不可见时发送系统通知
    const summaryName =
      targets.length === 1
        ? targets[0].name
        : `${targets[0].name} ${t("common.etc")} ${targets.length}${t("convert.filesUnit")}`;
    await history.add({
      kind: "convert",
      name: summaryName,
      inputs: targets.map((f) => f.path),
      outputs,
      ok: okCount === targets.length,
    });
    if (!(await getCurrentWindow().isVisible())) {
      await notifyDone(okCount, targets.length - okCount);
    }
  } finally {
    converting.value = false;
  }
}

/** 发送系统通知（转换完成提醒）；无权限时静默跳过 */
async function notifyDone(ok: number, fail: number) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) granted = (await requestPermission()) === "granted";
    if (!granted) return;
    sendNotification({
      title: t("convert.notifyTitle"),
      body: t(fail > 0 ? "convert.notifyBodyPartial" : "convert.notifyBodyAll", { ok, fail }),
    });
  } catch {
    /* 通知失败不影响转换结果 */
  }
}

/** 未安装 LibreOffice 时引导切换（由提示条按钮触发）；切换前先重新检测，用户可能刚安装完 */
async function switchToLibreOffice() {
  try {
    await engine.refresh();
  } catch {
    /* 检测失败时按未安装处理 */
  }
  if (engine.useLibreOffice()) {
    message.success(t("engine.msgSwitchedLo"));
  } else {
    message.warning(t("engine.msgMissingLo"), { duration: 5000 });
    void openUrl("https://www.libreoffice.org/download/");
  }
}

/** 仅重新检测 LibreOffice 是否已安装 */
async function redetectEngine() {
  try {
    const ok = await engine.refresh();
    message[ok ? "success" : "warning"](
      ok ? t("engine.msgDetected", { action: t("convert.goSwitch") }) : t("engine.msgNotDetected"),
      { duration: 4000 }
    );
  } catch {
    message.error(t("engine.msgDetectFailed"));
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
/* 面板头 */
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
/* 引擎提示条 */
.engine-tip {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 16px;
  padding: 10px 14px;
  border-radius: 10px;
  font-size: 13px;
}
.engine-tip.ok {
  background: var(--green-soft);
  color: var(--green);
}
.engine-tip.light {
  background: var(--accent-soft);
  color: var(--accent);
}
.engine-tip.warn {
  background: var(--orange-soft);
  color: var(--orange);
}
.tip-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.tip-dot.ok { background: var(--green); }
.tip-dot.light { background: var(--accent); }
.tip-dot.warn { background: var(--orange); }
.tip-path {
  font-size: 11px;
  opacity: 0.7;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 300px;
}
.tip-sub {
  font-size: 11px;
  opacity: 0.7;
}
.tip-btn {
  border: none;
  background: var(--orange);
  color: #fff;
  font-size: 12px;
  padding: 3px 12px;
  border-radius: 6px;
  cursor: pointer;
}
.tip-btn.ghost {
  background: var(--bg-tag);
  color: var(--text-sub);
}
.tip-btn:hover { opacity: 0.9; }
/* 上传区 */
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
.zone-icons {
  display: flex;
  align-items: center;
  gap: 14px;
}
.zone-badge {
  width: 52px;
  height: 52px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: 700;
}
.zone-badge.doc {
  background: var(--accent-soft);
  color: var(--accent);
}
.zone-badge.pdf {
  background: var(--red-soft);
  color: var(--red);
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
/* 文件夹批量添加 */
.batch-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 12px;
}
.batch-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  border: 1px dashed var(--border-strong);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 13px;
  padding: 6px 14px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.batch-btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}
.batch-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.batch-hint {
  font-size: 12px;
  color: var(--text-faint);
}
/* 文件列表 */
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
.fname {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--text-body);
}
.fext { color: var(--text-muted); font-size: 12px; }
.arrow { color: var(--text-faint); }
.unsupported { color: var(--red); font-size: 13px; }
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
/* 操作区 */
.action-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border-soft);
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
/* 转换进度条 */
.progress-wrap {
  margin-top: 14px;
  display: flex;
  align-items: center;
  gap: 12px;
}
.progress-bar {
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: var(--bg-tag);
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.3s ease;
}
.progress-text {
  font-size: 12px;
  color: var(--text-sub);
  white-space: nowrap;
}
/* 任务列表 */
.task-list {
  margin-top: 18px;
}
.task-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-sub);
  margin-bottom: 4px;
}
.task-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 6px;
  border-top: 1px solid var(--border-soft);
  font-size: 13px;
}
.task-status { font-size: 12px; white-space: nowrap; }
.link-btn {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
}
.link-btn:hover { text-decoration: underline; }
.task-error {
  color: var(--red);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
