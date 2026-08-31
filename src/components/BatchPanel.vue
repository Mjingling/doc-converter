<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("batch.title") }}</h2>
      <p>{{ t("batch.subtitle") }}</p>
    </div>

    <!-- 文件夹选择区 -->
    <div class="upload-zone" @click="pickFolder">
      <div v-if="!folder" class="zone-empty">
        <NIcon :component="FolderOpenOutline" :size="34" color="#e6494c" />
        <p class="zone-main">{{ t("batch.pickFolder") }}</p>
        <p class="zone-sub">{{ t("batch.pickFolderSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="FolderOpenOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="folder">{{ folder }}</span>
        <span class="size-tag">{{ t("batch.folderLabel") }}</span>
      </div>
    </div>

    <!-- 文件列表 -->
    <div v-if="files.length" class="file-section">
      <div class="file-toolbar">
        <label class="check-all">
          <input type="checkbox" :checked="allSelected" @change="toggleAll" />
          <span>{{ t("batch.files", { n: selected.length, total: files.length }) }}</span>
        </label>
        <button class="link-btn" @click="toggleAll">
          {{ allSelected ? t("batch.clearSelect") : t("batch.selectAll") }}
        </button>
      </div>
      <div class="file-list">
        <label v-for="f in files" :key="f" class="file-row">
          <input type="checkbox" :checked="selected.includes(f)" @change="toggleFile(f)" />
          <NIcon :component="DocumentTextOutline" :size="15" color="#e6494c" />
          <span class="fname" :title="f">{{ f.split(/[\\/]/).pop() }}</span>
        </label>
      </div>
    </div>

    <!-- 操作类型 + 参数 -->
    <div v-if="files.length" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("batch.opLabel") }}</label>
        <NRadioGroup v-model:value="op">
          <div class="op-options">
            <NRadioButton v-for="o in ops" :key="o.id" :value="o.id" :label="o.label" />
          </div>
        </NRadioGroup>
      </div>

      <!-- 旋转参数 -->
      <div v-if="op === 'rotate'" class="config-row">
        <label class="config-label">{{ t("rotate.angleLabel") }}</label>
        <NRadioGroup v-model:value="angle">
          <div class="op-options">
            <NRadioButton v-for="a in [90, 180, 270]" :key="a" :value="a" :label="`${a}°`" />
          </div>
        </NRadioGroup>
      </div>

      <!-- 水印参数 -->
      <template v-if="op === 'watermark'">
        <div class="config-row">
          <label class="config-label">{{ t("watermark.textLabel") }}</label>
          <NInput v-model:value="text" :placeholder="t('watermark.textPlaceholder')" />
        </div>
        <div class="config-row">
          <label class="config-label">{{ t("watermark.opacityLabel") }}</label>
          <NSlider v-model:value="opacity" :min="0.05" :max="1" :step="0.05" :format-tooltip="(v) => Math.round(v * 100) + '%'" />
        </div>
      </template>

      <!-- 加密参数 -->
      <template v-if="op === 'encrypt'">
        <div class="config-row">
          <label class="config-label">{{ t("encrypt.userPassLabel") }}</label>
          <NInput v-model:value="userPass" type="password" show-password-on="click" :placeholder="t('encrypt.userPassPlaceholder')" />
        </div>
        <div class="config-row">
          <label class="config-label">{{ t("encrypt.ownerPassLabel") }}</label>
          <NInput v-model:value="ownerPass" type="password" show-password-on="click" :placeholder="t('encrypt.ownerPassPlaceholder')" />
          <span class="config-hint">{{ t("encrypt.ownerHint") }}</span>
        </div>
      </template>

      <!-- 页码参数 -->
      <div v-if="op === 'pages'" class="config-row">
        <label class="config-label">{{ t("rotate.styleLabel") }}</label>
        <NRadioGroup v-model:value="style">
          <div class="op-options">
            <NRadioButton value="page" :label="t('rotate.stylePage')" />
            <NRadioButton value="pageOf" :label="t('rotate.stylePageOf')" />
          </div>
        </NRadioGroup>
      </div>

      <!-- 解密参数（统一密码模式） -->
      <div v-if="op === 'decrypt'" class="config-row">
        <label class="config-label">{{ t("encrypt.userPassLabel") }}</label>
        <NInput v-model:value="decryptPass" type="password" show-password-on="click" :placeholder="t('batch.decryptPassPlaceholder')" />
        <span class="config-hint">{{ t("batch.decryptHint") }}</span>
      </div>

    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t("batch.hint") }}</span>
      <button class="cta" :disabled="!files.length || running" @click="doBatch">
        <NIcon :component="CopyOutline" :size="17" />
        {{ running ? t("batch.running", { done, total }) : t("batch.run") }}
      </button>
    </div>

    <!-- 执行进度：多文件真实百分比 -->
    <TaskProgress
      :running="running"
      :progress="total > 0 ? Math.round((done / total) * 100) : 0"
      :label="t('batch.running', { done, total })"
    />
    <p v-if="running" class="progress-file">{{ t("batch.currentFile", { name: current }) }}</p>

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInput, NRadioButton, NRadioGroup, NSlider, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  ArchiveOutline, CopyOutline, DocumentTextOutline, FolderOpenOutline,
  ListOutline, LockClosedOutline, RefreshOutline, WaterOutline,
} from "@vicons/ionicons5";
import {
  openPath, pdfCompress, pdfDecrypt, pdfEncrypt, pdfExtractImages, pdfExtractTextToFile,
  pdfPageNumbers, pdfRotate, pdfWatermark, scanDirectory,
} from "../api";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";
import { useHistoryStore } from "../stores/history";
import { useSettingsStore } from "../stores/settings";
import { dirOf, defaultOutDir } from "../utils/file";
import { notifyDone } from "../utils/notify";
import { maybeAutoOpenOutput } from "../utils/autoOpen";

const { t } = useI18n();
const message = useMessage();
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");
const history = useHistoryStore();
const settings = useSettingsStore();

/** 批量操作类型 */
type Op = "rotate" | "watermark" | "encrypt" | "compress" | "pages" | "decrypt" | "extractText" | "extractImages";
const ops: { id: Op; label: string }[] = [
  { id: "rotate", label: t("batch.opRotate") },
  { id: "watermark", label: t("batch.opWatermark") },
  { id: "encrypt", label: t("batch.opEncrypt") },
  { id: "compress", label: t("batch.opCompress") },
  { id: "pages", label: t("batch.opPages") },
  { id: "decrypt", label: t("batch.opDecrypt") },
  { id: "extractText", label: t("batch.opExtractText") },
  { id: "extractImages", label: t("batch.opExtractImages") },
];

const folder = ref("");
/** 扫描到的全部 PDF */
const files = ref<string[]>([]);
/** 勾选参与处理的文件 */
const selected = ref<string[]>([]);
const op = ref<Op>("rotate");
const angle = ref(90);
const text = ref(t("watermark.defaultText"));
const opacity = ref(0.2);
const userPass = ref("");
const ownerPass = ref("");
const decryptPass = ref("");
const style = ref<"page" | "pageOf">("page");

const running = ref(false);
const done = ref(0);
const total = ref(0);
const current = ref("");

const allSelected = computed(() => files.value.length > 0 && selected.value.length === files.value.length);

async function pickFolder() {
  const dir = await openDialog({ directory: true });
  if (!dir) return;
  await loadFolder(String(dir));
}

/** 扫描文件夹中的 PDF 并载入列表 */
async function loadFolder(dir: string) {
  let pdfs: string[];
  try {
    pdfs = await scanDirectory(dir, ["pdf"]);
  } catch (e) {
    message.error(t("batch.scanFail", { err: String(e) }));
    return;
  }
  if (!pdfs.length) {
    message.warning(t("batch.noPdf"));
    return;
  }
  folder.value = dir;
  files.value = pdfs;
  selected.value = [...pdfs];
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop）：单个非 PDF 路径按文件夹扫描，PDF 直接加入 */
async function handleDrop(paths: string[]) {
  const pdfs = paths.filter((p) => /\.pdf$/i.test(p));
  const maybeDir = paths.filter((p) => !/\.pdf$/i.test(p));
  if (pdfs.length) {
    const merged = Array.from(new Set([...files.value, ...pdfs]));
    files.value = merged;
    selected.value = merged;
    if (!folder.value) folder.value = pdfs[0].split(/[\\/]/).slice(0, -1).join("/");
  }
  for (const d of maybeDir) {
    try {
      const found = await scanDirectory(d, ["pdf"]);
      if (found.length) {
        if (!folder.value) folder.value = d;
        const merged = Array.from(new Set([...files.value, ...found]));
        files.value = merged;
        selected.value = merged;
      }
    } catch {
      // 不是目录，忽略
    }
  }
}
defineExpose({ handleDrop });

function toggleAll() {
  selected.value = allSelected.value ? [] : [...files.value];
}
function toggleFile(f: string) {
  selected.value = selected.value.includes(f)
    ? selected.value.filter((x) => x !== f)
    : [...selected.value, f];
}

/** 按操作类型生成输出路径（默认源同目录，原名加后缀） */
function outputPathFor(f: string): string {
  const base = f.split(/[\\/]/).pop()?.replace(/\.pdf$/i, "") ?? "output";
  const dir = defaultOutDir(f, settings.defaultOutDir);
  // 提取文本输出 .txt；提取图片输出到子目录
  if (op.value === "extractText") return `${dir}/${base}.txt`;
  if (op.value === "extractImages") return `${dir}/${base}_images`;
  const suffix = {
    rotate: "_rotated",
    watermark: "_watermarked",
    encrypt: "_encrypted",
    compress: "_compressed",
    pages: "_numbered",
    decrypt: "_decrypted",
  }[op.value];
  return `${dir}/${base}${suffix}.pdf`;
}

/** 对单个文件执行当前操作，返回输出路径 */
async function runOne(f: string): Promise<string> {
  const out = outputPathFor(f);
  switch (op.value) {
    case "rotate":
      return pdfRotate(f, out, angle.value);
    case "watermark":
      // 批量水印暂未提供颜色/字号控件，使用与单文件面板一致的默认值（灰色 #808080、26pt）
      return pdfWatermark(f, out, text.value.trim(), opacity.value, [128, 128, 128], 26);
    case "encrypt":
      return pdfEncrypt(f, out, userPass.value, ownerPass.value || userPass.value);
    case "compress":
      return pdfCompress(f, out);
    case "pages":
      return pdfPageNumbers(f, out, style.value);
    case "decrypt":
      return pdfDecrypt(f, out, decryptPass.value);
    case "extractText":
      return pdfExtractTextToFile(f, out);
    case "extractImages":
      await pdfExtractImages(f, out);
      return out;
  }
}

async function doBatch() {
  const targets = selected.value;
  if (!targets.length) {
    message.warning(t("batch.warnNoSelect"));
    return;
  }
  if (op.value === "watermark" && !text.value.trim()) {
    message.warning(t("watermark.warnNoText"));
    return;
  }
  if (op.value === "encrypt" && !userPass.value) {
    message.warning(t("encrypt.warnNoPass"));
    return;
  }
  if (op.value === "decrypt" && !decryptPass.value) {
    message.warning(t("encrypt.warnNoPass"));
    return;
  }
  running.value = true;
  done.value = 0;
  total.value = targets.length;
  const outs: string[] = [];
  let failed = 0;
  for (const f of targets) {
    current.value = f.split(/[\\/]/).pop() ?? f;
    try {
      outs.push(await runOne(f));
    } catch {
      failed++;
    }
    done.value++;
  }
  running.value = false;

  const okCount = targets.length - failed;
  await history.add({
    kind: op.value,
    name: t("batch.resultName", { n: okCount }),
    inputs: targets,
    outputs: outs,
    ok: okCount > 0,
  });
  if (okCount > 0) {
    message.success(t("batch.success", { ok: okCount, fail: failed }), { duration: 5000 });
    void notifyDone(t("common.taskDone"), t("batch.success", { ok: okCount, fail: failed }));
    if (outs.length) void maybeAutoOpenOutput(outs[0]);
    resultText.value = t("batch.success", { ok: okCount, fail: failed });
    resultOutputs.value = outs;
    if (outs.length) {
      openPath(dirOf(outs[0])).catch(() => {
        /* 打开目录失败不影响结果提示 */
      });
    }
  } else {
    message.error(t("batch.allFail"));
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
.zone-sub {
  margin: 0;
  font-size: 12px;
  color: var(--text-faint);
}
.zone-filled {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  font-size: 14px;
  color: var(--text-sub);
}
.fname {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--text-body);
}
.size-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 8px;
  color: var(--text-muted);
  background: var(--bg-tag);
  flex-shrink: 0;
}
.file-section {
  margin-top: 16px;
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  overflow: hidden;
}
.file-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 14px;
  background: var(--bg-tag);
  font-size: 12px;
  color: var(--text-muted);
}
.check-all {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}
.file-list {
  max-height: 220px;
  overflow-y: auto;
  padding: 6px 14px;
}
.file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 0;
  cursor: pointer;
  font-size: 13px;
}
.file-row .fname {
  flex: 1;
  min-width: 0;
}
.link-btn {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 12px;
  cursor: pointer;
  padding: 2px 4px;
}
.link-btn:hover {
  text-decoration: underline;
}
.config {
  margin-top: 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
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
.config-hint {
  font-size: 12px;
  color: var(--text-faint);
}
.op-options {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.progress-file {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
.cta:hover:not(:disabled) {
  opacity: 0.85;
}
.cta:disabled {
  background: var(--cta-disabled);
  cursor: not-allowed;
}
</style>
