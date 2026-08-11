<template>
  <n-modal v-model:show="show" preset="card" :title="t('update.title')" style="width: 380px">
    <div class="update-body">
      <!-- 检查中 -->
      <div v-if="status === 'checking'" class="state-wrap">
        <n-spin size="small" />
        <span class="state-text">{{ t("update.checking") }}</span>
      </div>
      <!-- 已是最新版 -->
      <div v-else-if="status === 'latest'" class="state-wrap">
        <span class="icon-ok">
          <NIcon :component="CheckmarkCircleOutline" :size="28" color="#18a058" />
        </span>
        <span class="state-text">{{ t("update.latest") }}</span>
        <span class="version-tag">v{{ currentVersion }}</span>
      </div>
      <!-- 发现新版本 -->
      <div v-else-if="status === 'found'" class="state-wrap">
        <span class="icon-new">
          <NIcon :component="ArrowUpCircleOutline" :size="28" color="#2080f0" />
        </span>
        <div class="found-header">
          <span class="version-old">v{{ currentVersion }}</span>
          <NIcon :component="ArrowForwardOutline" :size="16" class="arrow" />
          <span class="version-new">v{{ info!.latestVersion }}</span>
        </div>
        <p v-if="info!.notes" class="release-notes">{{ info!.notes }}</p>
        <button class="download-btn" @click="doDownload">
          <NIcon :component="DownloadOutline" :size="16" />
          {{ t("update.download") }}
        </button>
      </div>
      <!-- 检查失败 -->
      <div v-else class="state-wrap">
        <span class="icon-err">
          <NIcon :component="WarningOutline" :size="28" color="#e6494c" />
        </span>
        <span class="state-text">{{ t("update.error") }}</span>
        <button class="retry-btn" @click="handleCheck">
          {{ t("retry") }}
        </button>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { NIcon, NModal, NSpin } from "naive-ui";
import { useI18n } from "vue-i18n";
import {
  CheckmarkCircleOutline,
  ArrowUpCircleOutline,
  ArrowForwardOutline,
  DownloadOutline,
  WarningOutline,
} from "@vicons/ionicons5";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { checkUpdate } from "../api";
import type { UpdateInfo } from "../api";

const { t } = useI18n();
const show = defineModel<boolean>("show", { default: false });

/** 当前状态：checking / latest / found / error */
const status = ref<"checking" | "latest" | "found" | "error" | "idle">("idle");
/** 检查结果（仅在 found 时有效） */
const info = ref<UpdateInfo | null>(null);
/** 当前版本号（从 Tauri 运行时读取） */
const currentVersion = ref("");

async function handleCheck() {
  status.value = "checking";
  info.value = null;
  try {
    currentVersion.value = await getVersion();
    const result = await checkUpdate(currentVersion.value);
    if (!result) {
      status.value = "error";
    } else if (result.hasUpdate) {
      status.value = "found";
      info.value = result;
    } else {
      status.value = "latest";
    }
  } catch {
    status.value = "error";
  }
}

/** 弹窗打开时自动检查 */
watch(show, (v) => {
  if (v) void handleCheck();
});

/** 前往下载页面 */
function doDownload() {
  if (info.value?.downloadUrl) {
    void openUrl(info.value.downloadUrl);
  }
}
</script>

<style scoped>
.update-body {
  min-height: 100px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}
.state-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 12px 0;
}
.state-text {
  font-size: 14px;
  color: var(--text-sub);
}
.version-tag {
  font-size: 13px;
  color: var(--text-muted);
  background: var(--bg-tag);
  padding: 2px 10px;
  border-radius: 8px;
}
.found-header {
  display: flex;
  align-items: center;
  gap: 8px;
}
.version-old {
  font-size: 14px;
  color: var(--text-muted);
  text-decoration: line-through;
}
.version-new {
  font-size: 18px;
  font-weight: 700;
  color: var(--accent);
}
.arrow {
  color: var(--text-faint);
}
.release-notes {
  margin: 0;
  font-size: 13px;
  color: var(--text-sub);
  white-space: pre-wrap;
  text-align: center;
  max-width: 320px;
  line-height: 1.6;
}
.download-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: none;
  background: var(--accent);
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  padding: 8px 22px;
  border-radius: 8px;
  cursor: pointer;
  transition: opacity 0.15s;
}
.download-btn:hover {
  opacity: 0.85;
}
.retry-btn {
  border: 1px solid var(--border-strong);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 13px;
  padding: 6px 18px;
  border-radius: 6px;
  cursor: pointer;
}
.retry-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}
</style>