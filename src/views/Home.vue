<script setup lang="ts">
import { nextTick, onMounted, ref } from "vue";
import { useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { listen } from "@tauri-apps/api/event";
import { getLaunchFiles } from "../api";
import SideNav from "../components/SideNav.vue";
import CommandPalette from "../components/CommandPalette.vue";
import QuickDropModal from "../components/QuickDropModal.vue";
import TitleBar from "../components/TitleBar.vue";
import SettingsPanel from "../components/SettingsPanel.vue";
import MergePanel from "../components/MergePanel.vue";
import SplitPanel from "../components/SplitPanel.vue";
import CompressPanel from "../components/CompressPanel.vue";
import OrganizePanel from "../components/OrganizePanel.vue";
import WatermarkPanel from "../components/WatermarkPanel.vue";
import RotatePanel from "../components/RotatePanel.vue";
import EncryptPanel from "../components/EncryptPanel.vue";
import ImagesToPdfPanel from "../components/ImagesToPdfPanel.vue";
import BatchPanel from "../components/BatchPanel.vue";
import MetadataPanel from "../components/MetadataPanel.vue";
import CropPanel from "../components/CropPanel.vue";
import OutlinePanel from "../components/OutlinePanel.vue";
import DocxExtractPanel from "../components/DocxExtractPanel.vue";
import ImageCompressPanel from "../components/ImageCompressPanel.vue";
import ImageConvertPanel from "../components/ImageConvertPanel.vue";
import PdfRenderPanel from "../components/PdfRenderPanel.vue";
import SignaturePanel from "../components/SignaturePanel.vue";
import ExtractPdfImagesPanel from "../components/ExtractPdfImagesPanel.vue";
import RemoveWatermarkPanel from "../components/RemoveWatermarkPanel.vue";
import ComparePdfPanel from "../components/ComparePdfPanel.vue";
import WebToPdfPanel from "../components/WebToPdfPanel.vue";
import BatchRenamePanel from "../components/BatchRenamePanel.vue";
import AiSummaryPanel from "../components/AiSummaryPanel.vue";
import AiAssistantPanel from "../components/AiAssistantPanel.vue";
import AiDocQaPanel from "../components/AiDocQaPanel.vue";
import TranslatePanel from "../components/TranslatePanel.vue";
import HistoryPanel from "../components/HistoryPanel.vue";
import ConvertPanel from "../components/ConvertPanel.vue";
import { useEngineStore } from "../stores/engine";
import { useHistoryStore } from "../stores/history";
import type { ConvertScene, NavId } from "../types";
import { flattenNavItems } from "../navItems";

const engine = useEngineStore();
const message = useMessage();
const { t } = useI18n();
const history = useHistoryStore();

/** 当前激活的导航功能（默认进入 AI 助手） */
const active = ref<NavId>("aiAssistant");

/**
 * 转换场景配置（文档转换类导航项共用 ConvertPanel）
 * - 固定目标格式的场景：pdf2word / pdf2image / word2pdf / excel2pdf / ppt2pdf
 * - 动态获取目标格式的场景：convert（文档互转）
 */
type ConvertSceneNavId = Exclude<
  NavId,
  "merge" | "split" | "compress" | "organize" | "watermark" | "rotate" | "encrypt" | "images2pdf" | "batch"
  | "metadata" | "crop" | "outline" | "docxExtract" | "imageCompress" | "imageConvert" | "pdfRender" | "signature"
  | "pdfExtractImages" | "removeWatermark" | "comparePdf" | "webToPdf" | "batchRename" | "aiSummary" | "aiAssistant" | "docQa" | "translate" | "settings" | "history"
>;
const convertScenes: Record<ConvertSceneNavId, ConvertScene> = {
  pdf2word: {
    title: "scenes.pdf2word.title",
    subtitle: "scenes.pdf2word.subtitle",
    acceptExts: ["pdf"],
    fixedTargets: [{ ext: "docx", label: "convert.targetDocx" }],
    engineRequired: true,
  },
  pdf2excel: {
    title: "scenes.pdf2excel.title",
    subtitle: "scenes.pdf2excel.subtitle",
    acceptExts: ["pdf"],
    fixedTargets: [{ ext: "xlsx", label: "convert.targetXlsx" }],
    engineRequired: true,
  },
  pdf2image: {
    title: "scenes.pdf2image.title",
    subtitle: "scenes.pdf2image.subtitle",
    acceptExts: ["pdf"],
    fixedTargets: [
      { ext: "png", label: "convert.targetPng" },
      { ext: "jpg", label: "convert.targetJpg" },
    ],
    engineRequired: true,
  },
  word2pdf: {
    title: "scenes.word2pdf.title",
    subtitle: "scenes.word2pdf.subtitle",
    acceptExts: ["doc", "docx", "odt", "rtf", "txt", "html", "md", "epub"],
    fixedTargets: [{ ext: "pdf", label: "convert.targetPdf" }],
    engineRequired: true,
  },
  excel2pdf: {
    title: "scenes.excel2pdf.title",
    subtitle: "scenes.excel2pdf.subtitle",
    acceptExts: ["xls", "xlsx", "ods", "csv"],
    fixedTargets: [{ ext: "pdf", label: "convert.targetPdf" }],
    engineRequired: true,
  },
  ppt2pdf: {
    title: "scenes.ppt2pdf.title",
    subtitle: "scenes.ppt2pdf.subtitle",
    acceptExts: ["ppt", "pptx", "odp"],
    fixedTargets: [{ ext: "pdf", label: "convert.targetPdf" }],
    engineRequired: true,
  },
  convert: {
    title: "scenes.convert.title",
    subtitle: "scenes.convert.subtitle",
    acceptExts: ["pdf", "doc", "docx", "odt", "rtf", "txt", "html", "md", "epub", "xls", "xlsx", "ods", "csv", "ppt", "pptx", "odp"],
    engineRequired: true,
  },
};

/* ---------- 面板引用（拖拽事件按当前导航项分发） ---------- */
const mergeRef = ref<InstanceType<typeof MergePanel> | null>(null);
const splitRef = ref<InstanceType<typeof SplitPanel> | null>(null);
const compressRef = ref<InstanceType<typeof CompressPanel> | null>(null);
const organizeRef = ref<InstanceType<typeof OrganizePanel> | null>(null);
const watermarkRef = ref<InstanceType<typeof WatermarkPanel> | null>(null);
const rotateRef = ref<InstanceType<typeof RotatePanel> | null>(null);
const encryptRef = ref<InstanceType<typeof EncryptPanel> | null>(null);
const images2pdfRef = ref<InstanceType<typeof ImagesToPdfPanel> | null>(null);
const batchRef = ref<InstanceType<typeof BatchPanel> | null>(null);
const metadataRef = ref<InstanceType<typeof MetadataPanel> | null>(null);
const cropRef = ref<InstanceType<typeof CropPanel> | null>(null);
const outlineRef = ref<InstanceType<typeof OutlinePanel> | null>(null);
const docxExtractRef = ref<InstanceType<typeof DocxExtractPanel> | null>(null);
const imageCompressRef = ref<InstanceType<typeof ImageCompressPanel> | null>(null);
const imageConvertRef = ref<InstanceType<typeof ImageConvertPanel> | null>(null);
const pdfRenderRef = ref<InstanceType<typeof PdfRenderPanel> | null>(null);
const signatureRef = ref<InstanceType<typeof SignaturePanel> | null>(null);
const pdfExtractImagesRef = ref<InstanceType<typeof ExtractPdfImagesPanel> | null>(null);
const removeWatermarkRef = ref<InstanceType<typeof RemoveWatermarkPanel> | null>(null);
const comparePdfRef = ref<InstanceType<typeof ComparePdfPanel> | null>(null);
const webToPdfRef = ref<InstanceType<typeof WebToPdfPanel> | null>(null);
const batchRenameRef = ref<InstanceType<typeof BatchRenamePanel> | null>(null);
const aiSummaryRef = ref<InstanceType<typeof AiSummaryPanel> | null>(null);
const aiAssistantRef = ref<InstanceType<typeof AiAssistantPanel> | null>(null);
const docQaRef = ref<InstanceType<typeof AiDocQaPanel> | null>(null);
const translateRef = ref<InstanceType<typeof TranslatePanel> | null>(null);
const convertRef = ref<InstanceType<typeof ConvertPanel> | null>(null);

/** 按当前导航项把 tauri://drag-drop 的文件路径分发给对应面板 */
function dispatchDrop(paths: string[]) {
  switch (active.value) {
    case "merge":
      mergeRef.value?.handleDrop(paths);
      break;
    case "split":
      splitRef.value?.handleDrop(paths);
      break;
    case "compress":
      compressRef.value?.handleDrop(paths);
      break;
    case "organize":
      organizeRef.value?.handleDrop(paths);
      break;
    case "watermark":
      watermarkRef.value?.handleDrop(paths);
      break;
    case "rotate":
      rotateRef.value?.handleDrop(paths);
      break;
    case "encrypt":
      encryptRef.value?.handleDrop(paths);
      break;
    case "images2pdf":
      images2pdfRef.value?.handleDrop(paths);
      break;
    case "batch":
      batchRef.value?.handleDrop(paths);
      break;
    case "metadata":
      metadataRef.value?.handleDrop(paths[0]);
      break;
    case "crop":
      cropRef.value?.handleDrop(paths[0]);
      break;
    case "outline":
      outlineRef.value?.handleDrop(paths[0]);
      break;
    case "docxExtract":
      docxExtractRef.value?.handleDrop(paths[0]);
      break;
    case "imageCompress":
      imageCompressRef.value?.handleDrop(paths);
      break;
    case "imageConvert":
      imageConvertRef.value?.handleDrop(paths);
      break;
    case "pdfRender":
      pdfRenderRef.value?.handleDrop(paths);
      break;
    case "signature":
      signatureRef.value?.handleDrop(paths);
      break;
    case "pdfExtractImages":
      pdfExtractImagesRef.value?.handleDrop(paths[0]);
      break;
    case "removeWatermark":
      removeWatermarkRef.value?.handleDrop(paths[0]);
      break;
    case "comparePdf":
      comparePdfRef.value?.handleDrop(paths);
      break;
    case "webToPdf":
      message.warning(t("webToPdf.warnNoDrag"))
      break;
    case "batchRename":
      batchRenameRef.value?.handleDrop(paths);
      break;
    case "aiSummary":
      aiSummaryRef.value?.handleDrop(paths);
      break;
    case "aiAssistant":
      aiAssistantRef.value?.handleDrop(paths);
      break;
    case "docQa":
      docQaRef.value?.handleDrop(paths);
      break;
    case "translate":
      translateRef.value?.handleDrop(paths);
      break;
    default:
      convertRef.value?.handleDrop(paths);
  }
}

// Tauri 内建拖拽事件提供文件路径；HTML5 drop 拿不到路径，仅阻止默认行为
function preventDefault(e: Event) {
  e.preventDefault();
}

/** 外部唤起（Finder 右键「用 DocMorph 打开」）：把文件路径分发给当前面板并提示 */
function handleExternalFiles(files: string[]) {
  dispatchDrop(files);
  message.info(t("common.filesOpened", { n: files.length }), { duration: 3000 });
}

/* ---------- 拖拽直达：未落在面板上传区时弹出 QuickDropModal ---------- */
const showQuickDrop = ref(false);
const quickDropPaths = ref<string[]>([]);

/** 按落点判断：命中上传区则分发给当前面板，否则弹出快捷操作弹窗 */
function handleWindowDrop(paths: string[], position?: { x: number; y: number }) {
  if (position) {
    const el = document.elementFromPoint(position.x, position.y);
    if (el?.closest(".upload-zone")) {
      dispatchDrop(paths);
      return;
    }
  }
  quickDropPaths.value = paths;
  showQuickDrop.value = true;
}

/** 弹窗选择「AI 摘要」：切到摘要面板并传入文件 */
function openSummary(paths: string[]) {
  active.value = "aiSummary";
  void nextTick(() => aiSummaryRef.value?.handleDrop(paths));
}

onMounted(async () => {
  engine.refresh().catch(() => {
    /* 引擎检测失败时保持内置模式 */
  });
  // 先注册事件监听，再拉取启动文件，避免 TOCTOU 竞态
  const unlisten = listen<{ paths: string[]; position?: { x: number; y: number } }>("tauri://drag-drop", (e) => {
    handleWindowDrop(e.payload.paths, e.payload.position);
  });
  // 应用已运行时的 Finder 唤起（单实例插件转发）
  const unlistenOpen = listen<string[]>("open-files", (e) => {
    handleExternalFiles(e.payload);
  });
  // 文件夹监控自动转换结果
  const unlistenWatcher = listen<{ input: string; output?: string; ok: boolean; error?: string }>("watcher-event", (e) => {
    const { input, output, ok, error } = e.payload;
    const name = input.split(/[/\\]/).pop() || input;
    history.add({
      kind: "watcher",
      name,
      inputs: [input],
      outputs: ok && output ? [output] : [],
      ok,
    });
    if (ok) {
      message.success(t("settings.watcherDone", { name }), { duration: 4000 });
    } else {
      message.error(t("settings.watcherFail", { name, err: error ?? "" }), { duration: 6000 });
    }
  });
  // 托盘菜单「设置…」：切换到设置面板
  const unlistenSettings = listen("open-settings", () => {
    active.value = "settings";
  });
  // 桌面宠物右键菜单：切到 AI 助手面板
  const unlistenAssistant = listen("open-assistant", () => {
    active.value = "aiAssistant";
  });
  // 桌面宠物双击快捷面板：切到指定功能面板（非法 id 兜底回转换）
  const unlistenPanel = listen<string>("open-panel", (e) => {
    active.value = flattenNavItems().some((i) => i.id === e.payload) ? (e.payload as NavId) : "convert";
  });
  // 然后拉取启动参数中的文件路径并分发
  const launchFiles = await getLaunchFiles();
  if (launchFiles.length) handleExternalFiles(launchFiles);
  window.addEventListener("dragover", preventDefault);
  window.addEventListener("drop", preventDefault);
  return () => {
    unlisten.then((f) => f());
    unlistenOpen.then((f) => f());
    unlistenWatcher.then((f) => f());
    unlistenSettings.then((f) => f());
    unlistenAssistant.then((f) => f());
    unlistenPanel.then((f) => f());
    window.removeEventListener("dragover", preventDefault);
    window.removeEventListener("drop", preventDefault);
  };
});
</script>

<template>
  <div class="layout">
    <!-- 自定义标题栏（所有平台） -->
    <TitleBar />

    <div class="layout-body">
      <!-- 左侧导航 -->
      <SideNav :active="active" @select="(id: NavId) => (active = id)" />
      <CommandPalette :active="active" @select="(id: NavId) => (active = id)" />
      <QuickDropModal
        :visible="showQuickDrop"
        :paths="quickDropPaths"
        @close="showQuickDrop = false"
        @goto-summary="openSummary"
      />

      <!-- 右侧内容区 -->
      <main class="content">
        <MergePanel v-if="active === 'merge'" ref="mergeRef" />
        <SplitPanel v-else-if="active === 'split'" ref="splitRef" />
        <CompressPanel v-else-if="active === 'compress'" ref="compressRef" />
        <OrganizePanel v-else-if="active === 'organize'" ref="organizeRef" />
        <WatermarkPanel v-else-if="active === 'watermark'" ref="watermarkRef" />
        <RotatePanel v-else-if="active === 'rotate'" ref="rotateRef" />
        <EncryptPanel v-else-if="active === 'encrypt'" ref="encryptRef" />
        <ImagesToPdfPanel v-else-if="active === 'images2pdf'" ref="images2pdfRef" />
        <BatchPanel v-else-if="active === 'batch'" ref="batchRef" />
        <MetadataPanel v-else-if="active === 'metadata'" ref="metadataRef" />
        <CropPanel v-else-if="active === 'crop'" ref="cropRef" />
        <OutlinePanel v-else-if="active === 'outline'" ref="outlineRef" />
        <DocxExtractPanel v-else-if="active === 'docxExtract'" ref="docxExtractRef" />
        <ImageCompressPanel v-else-if="active === 'imageCompress'" ref="imageCompressRef" />
        <ImageConvertPanel v-else-if="active === 'imageConvert'" ref="imageConvertRef" />
        <PdfRenderPanel v-else-if="active === 'pdfRender'" ref="pdfRenderRef" />
        <SignaturePanel v-else-if="active === 'signature'" ref="signatureRef" />
        <ExtractPdfImagesPanel v-else-if="active === 'pdfExtractImages'" ref="pdfExtractImagesRef" />
        <RemoveWatermarkPanel v-else-if="active === 'removeWatermark'" ref="removeWatermarkRef" />
        <ComparePdfPanel v-else-if="active === 'comparePdf'" ref="comparePdfRef" />
        <WebToPdfPanel v-else-if="active === 'webToPdf'" ref="webToPdfRef" />
        <BatchRenamePanel v-else-if="active === 'batchRename'" ref="batchRenameRef" />
        <AiSummaryPanel v-else-if="active === 'aiSummary'" ref="aiSummaryRef" />
        <AiAssistantPanel v-else-if="active === 'aiAssistant'" ref="aiAssistantRef" />
        <AiDocQaPanel v-else-if="active === 'docQa'" ref="docQaRef" />
        <TranslatePanel v-else-if="active === 'translate'" ref="translateRef" />
        <HistoryPanel v-else-if="active === 'history'" />
        <SettingsPanel v-else-if="active === 'settings'" />
        <ConvertPanel
          v-else
          :key="active"
          ref="convertRef"
          :scene="convertScenes[active as ConvertSceneNavId]"
        />
      </main>
    </div>

    <!-- 设置入口：左下角设置菜单 / 托盘菜单 open-settings 事件 -->
  </div>
</template>

<style scoped>
.layout {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-page);
}
.layout-body {
  flex: 1;
  display: flex;
  min-height: 0;
}
.content {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
  box-sizing: border-box;
  min-width: 0;
}
</style>
