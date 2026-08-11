<template>
  <div class="settings-panel">
    <div class="panel-head">
      <NIcon :component="SettingsOutline" :size="22" color="var(--accent)" />
      <h2>{{ t("settings.title") }}</h2>
    </div>
    <n-tabs v-model:value="tab" type="segment" size="small" class="settings-tabs">
      <!-- 通用设置 -->
      <n-tab-pane name="general" :tab="t('settings.tabGeneral')">
        <!-- 默认输出目录 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="FolderOpenOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.outDir") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <div class="ctrl-value" :class="{ empty: !settings.defaultOutDir }" :title="settings.defaultOutDir">
                  {{ settings.defaultOutDir || t("settings.notSet") }}
                </div>
                <button class="mini-btn primary" @click="chooseDir">{{ t("settings.choose") }}</button>
                <button v-if="settings.defaultOutDir" class="mini-btn ghost" @click="clearDir">{{ t("settings.clear") }}</button>
              </div>
              <p class="card-hint">{{ t("settings.hint") }}</p>
            </div>
          </div>
        </div>

        <!-- 语言 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="GlobeOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.language") }}</span>
            </div>
            <div class="card-content">
              <NSelect :value="settings.locale" :options="localeOptions" size="small" @update:value="onLocaleChange" />
            </div>
          </div>
        </div>

        <!-- 主题 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="ColorPaletteOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.theme") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <span class="theme-sys-label">{{ t("settings.followSystem") }}</span>
                <NSwitch :value="settings.theme === 'system'" size="small" @update:value="onThemeFollowToggle" />
              </div>
              <div class="theme-picker">
                <div
                  class="theme-card"
                  :class="{ active: settings.theme === 'light' }"
                  @click="onThemeCardClick('light')"
                >
                  <div class="theme-preview theme-light-preview">
                    <div class="kb-row"><span></span><span></span><span></span><span></span></div>
                    <div class="kb-row"><span></span><span></span><span></span><span></span></div>
                    <div class="kb-row"><span></span><span class="wide"></span><span></span></div>
                  </div>
                  <div class="theme-check">
                    <NIcon v-if="settings.theme === 'light'" :component="CheckmarkOutline" :size="16" />
                  </div>
                  <span class="theme-name">{{ t("settings.light") }}</span>
                </div>
                <div
                  class="theme-card"
                  :class="{ active: settings.theme === 'dark' }"
                  @click="onThemeCardClick('dark')"
                >
                  <div class="theme-preview theme-dark-preview">
                    <div class="kb-row"><span></span><span></span><span></span><span></span></div>
                    <div class="kb-row"><span></span><span></span><span></span><span></span></div>
                    <div class="kb-row"><span></span><span class="wide"></span><span></span></div>
                  </div>
                  <div class="theme-check">
                    <NIcon v-if="settings.theme === 'dark'" :component="CheckmarkOutline" :size="16" />
                  </div>
                  <span class="theme-name">{{ t("settings.dark") }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 开机启动 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="PowerOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.autostart") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="autostart" size="small" @update:value="onAutostartChange" />
              </div>
              <p class="card-hint">{{ t("settings.autostartHint") }}</p>
            </div>
          </div>
        </div>
      </n-tab-pane>

      <!-- 高级设置 -->
      <n-tab-pane name="advanced" :tab="t('settings.tabAdvanced')">
        <!-- 文件夹监控 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="EyeOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.watcherTitle") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="settings.watcher.enabled" size="small" :disabled="watcherBusy" @update:value="onWatcherToggle" />
              </div>
              <p class="card-hint">{{ t("settings.watcherHint") }}</p>
              <div class="ctrl-row">
                <div class="ctrl-value" :class="{ empty: !settings.watcher.folder }" :title="settings.watcher.folder">
                  {{ settings.watcher.folder || t("settings.notSet") }}
                </div>
                <button class="mini-btn primary" @click="chooseWatcherFolder">{{ t("settings.choose") }}</button>
                <button v-if="settings.watcher.folder" class="mini-btn ghost" @click="clearWatcherFolder">{{ t("settings.clear") }}</button>
              </div>
              <div class="watcher-rules">
                <div v-for="rule in RULES" :key="rule.key" class="rule-row">
                  <span class="rule-label">{{ t(rule.labelKey) }}</span>
                  <NSelect
                    size="small"
                    :value="effectiveTargets[rule.key]"
                    :options="rule.options.map((o) => ({ label: o.toUpperCase(), value: o }))"
                    @update:value="(v: string) => onRuleChange(rule.key, v)"
                  />
                </div>
              </div>
              <p class="card-hint">{{ t("settings.watcherNeedLo") }}</p>
              <p class="card-hint">{{ t("settings.watcherOutDirHint") }}</p>
            </div>
          </div>
        </div>
      </n-tab-pane>

      <!-- AI 能力 -->
      <n-tab-pane name="ai" :tab="t('settings.tabAi')">
        <!-- 引擎模式 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="SparklesOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.aiMode") }}</span>
            </div>
            <div class="card-content">
              <div class="ai-mode-picker">
                <div
                  class="theme-card"
                  :class="{ active: settings.ai.mode === 'auto' }"
                  @click="onAiModeChange('auto')"
                >
                  <div class="mode-icon-wrap">
                    <NIcon :component="SparklesOutline" :size="22" />
                  </div>
                  <div class="theme-check">
                    <NIcon v-if="settings.ai.mode === 'auto'" :component="CheckmarkOutline" :size="16" />
                  </div>
                  <span class="theme-name">{{ t("settings.aiModeAuto") }}</span>
                </div>
                <div
                  class="theme-card"
                  :class="{ active: settings.ai.mode === 'local' }"
                  @click="onAiModeChange('local')"
                >
                  <div class="mode-icon-wrap">
                    <NIcon :component="CubeOutline" :size="22" />
                  </div>
                  <div class="theme-check">
                    <NIcon v-if="settings.ai.mode === 'local'" :component="CheckmarkOutline" :size="16" />
                  </div>
                  <span class="theme-name">{{ t("settings.aiModeLocal") }}</span>
                </div>
                <div
                  class="theme-card"
                  :class="{ active: settings.ai.mode === 'cloud' }"
                  @click="onAiModeChange('cloud')"
                >
                  <div class="mode-icon-wrap">
                    <NIcon :component="CloudOutline" :size="22" />
                  </div>
                  <div class="theme-check">
                    <NIcon v-if="settings.ai.mode === 'cloud'" :component="CheckmarkOutline" :size="16" />
                  </div>
                  <span class="theme-name">{{ t("settings.aiModeCloud") }}</span>
                </div>
                <div
                  class="theme-card"
                  :class="{ active: settings.ai.mode === 'local-server' }"
                  @click="onAiModeChange('local-server')"
                >
                  <div class="mode-icon-wrap">
                    <NIcon :component="DesktopOutline" :size="22" />
                  </div>
                  <div class="theme-check">
                    <NIcon v-if="settings.ai.mode === 'local-server'" :component="CheckmarkOutline" :size="16" />
                  </div>
                  <span class="theme-name">{{ t("settings.aiModeLocalServer") }}</span>
                </div>
              </div>
              <p class="card-hint">{{ t(aiModeHintKey) }}</p>
            </div>
          </div>
        </div>

        <!-- 云端 API 配置（cloud / auto 模式显示） -->
        <div v-if="settings.ai.mode === 'cloud' || settings.ai.mode === 'auto'" class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="CloudOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.aiCloudSection") }}</span>
            </div>
            <div class="card-content">
              <NInput
                size="small"
                :value="settings.ai.cloud.baseUrl"
                :placeholder="t('settings.aiBaseUrlPlaceholder')"
                @update:value="(v: string) => onCloudChange('baseUrl', v)"
              />
              <NInput
                size="small"
                type="password"
                show-password-on="click"
                :value="settings.ai.cloud.apiKey"
                :placeholder="t('settings.aiApiKeyPlaceholder')"
                @update:value="(v: string) => onCloudChange('apiKey', v)"
              />
              <div class="ctrl-row">
                <div class="ctrl-col">
                  <div class="col-label">{{ t("settings.aiEmbeddingModel") }}</div>
                  <NInput
                    size="small"
                    :value="settings.ai.cloud.embeddingModel"
                    @update:value="(v: string) => onCloudChange('embeddingModel', v)"
                  />
                </div>
                <div class="ctrl-col">
                  <div class="col-label">{{ t("settings.aiChatModel") }}</div>
                  <NInput
                    size="small"
                    :value="settings.ai.cloud.chatModel"
                    @update:value="(v: string) => onCloudChange('chatModel', v)"
                  />
                </div>
              </div>
              <p class="card-hint">{{ t("settings.aiCloudHint") }}</p>
              <NButton
                size="small"
                :loading="testing"
                :disabled="!settings.ai.cloud.baseUrl || !settings.ai.cloud.apiKey"
                @click="testCloud"
              >{{ t("settings.aiTest") }}</NButton>
            </div>
          </div>
        </div>

        <!-- 本地服务配置（local-server 模式显示） -->
        <div v-if="settings.ai.mode === 'local-server'" class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="DesktopOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.aiLocalServerSection") }}</span>
            </div>
            <div class="card-content">
              <NInput
                size="small"
                :value="settings.ai.localServer.baseUrl"
                :placeholder="t('settings.aiLocalServerBaseUrlPlaceholder')"
                @update:value="(v: string) => onLocalServerChange('baseUrl', v)"
              />
              <div class="ctrl-row">
                <div class="ctrl-col">
                  <div class="col-label">{{ t("settings.aiChatModel") }}</div>
                  <NInput
                    size="small"
                    :value="settings.ai.localServer.chatModel"
                    :placeholder="t('settings.aiLocalServerModelPlaceholder')"
                    @update:value="(v: string) => onLocalServerChange('chatModel', v)"
                  />
                </div>
                <div class="ctrl-col">
                  <div class="col-label">{{ t("settings.aiEmbeddingModel") }}</div>
                  <NInput
                    size="small"
                    :value="settings.ai.localServer.embeddingModel"
                    :placeholder="t('settings.aiLocalServerModelPlaceholder')"
                    @update:value="(v: string) => onLocalServerChange('embeddingModel', v)"
                  />
                </div>
              </div>
              <p class="card-hint">{{ t("settings.aiLocalServerHint") }}</p>
              <NButton
                size="small"
                :loading="localServerTesting"
                :disabled="!settings.ai.localServer.baseUrl"
                @click="testLocalServer"
              >{{ t("settings.aiTest") }}</NButton>
            </div>
          </div>
        </div>

        <!-- 本地 AI 模型（简化：仅显示状态 + 操作） -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="CubeOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.aiLocalSection") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <span>{{ t("settings.aiLocalChatStatus") }}</span>
                <span v-if="chatSize > 0" class="size-badge">{{ formatBytes(chatSize) }}</span>
                <span class="local-status" :class="chatStatus">{{ chatStatusText }}</span>
              </div>
              <div v-if="chatProgress" class="chat-progress">
                <NProgress :percentage="chatProgress.percent" :indicator-placement="'inside'" processing />
                <span class="progress-file">{{ chatProgress.file }}（{{ formatBytes(chatProgress.loaded) }} / {{ formatBytes(chatProgress.total) }}）</span>
              </div>
              <div class="ctrl-row">
                <NButton size="small" type="primary" ghost :loading="chatBusy" :disabled="chatStatus === 'downloading'" @click="downloadChat">
                  {{ t("settings.aiLocalChatDownload") }}
                </NButton>
                <NPopconfirm :positive-text="t('settings.ok')" :negative-text="t('settings.cancel')" @positive-click="deleteChat">
                  <template #trigger>
                    <NButton size="small" type="error" ghost :disabled="chatStatus === 'downloading' || chatBusy">
                      {{ t("settings.aiLocalChatDelete") }}
                    </NButton>
                  </template>
                  {{ t("settings.aiLocalChatDeleteConfirm") }}
                </NPopconfirm>
                <NButton size="small" quaternary :disabled="chatBusy" @click="refreshChatStatus">{{ t("settings.refresh") }}</NButton>
              </div>
              <p class="card-hint">{{ t("settings.aiLocalHint") }}</p>
            </div>
          </div>
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NButton, NIcon, NInput, NPopconfirm, NProgress, NSelect, NSwitch, NTabPane, NTabs, useMessage } from "naive-ui";
import {
  CheckmarkOutline, CloudOutline, ColorPaletteOutline, CubeOutline,
  DesktopOutline,
  EyeOutline, FolderOpenOutline, GlobeOutline, PowerOutline, SettingsOutline, SparklesOutline,
} from "@vicons/ionicons5";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { watcherStart, watcherStop } from "../api";
import { useSettingsStore } from "../stores/settings";
import type { AiMode, AppLocale, AppTheme } from "../stores/settings";
import { localEngineStatus, syncCloudConfig, syncLocalChatModel, localChatModelStatus, downloadLocalChatModel, deleteLocalChatModel, localChatModelSize, formatBytes, CloudProvider, syncLocalServerConfig } from "../ai";
import type { ChatModelProgress, ChatModelState } from "../ai";

const { t } = useI18n();
const settings = useSettingsStore();
const message = useMessage();

/** 根据当前 AI 模式显示对应的提示文案 */
const aiModeHintKey = computed(() => {
  switch (settings.ai.mode) {
    case "local": return "settings.aiModeHintLocal";
    case "local-server": return "settings.aiModeHintLocalServer";
    case "cloud": return "settings.aiModeHintCloud";
    default: return "settings.aiModeHintAuto";
  }
});

/** 开机启动当前状态（启动时从系统读取） */
const autostart = ref(false);

/** 当前页签：通用设置 / 高级设置 */
const tab = ref("general");

/** 监控操作进行中（防止重复启停） */
const watcherBusy = ref(false);

/** 本地 AI 引擎状态：unavailable（未加载）/ loading / ready */
const localStatus = ref<"unavailable" | "loading" | "ready">("unavailable");

/** 本地生成式模型状态：unavailable（未下载）/ downloading / ready */
const chatStatus = ref<ChatModelState>("unavailable");
/** 本地生成式模型缓存大小（字节） */
const chatSize = ref(0);
/** 下载进行中（pipeline 加载/推理会占用主线程，防止重复点击） */
const chatBusy = ref(false);
/** 下载进度（null = 未在下载） */
const chatProgress = ref<ChatModelProgress | null>(null);

/** 本地模型状态文案 */
const localStatusText = computed(() => {
  switch (localStatus.value) {
    case "ready": return t("settings.aiLocalReady");
    case "loading": return t("settings.aiLocalLoading");
    default: return t("settings.aiLocalNotReady");
  }
});

/** 切换 AI 引擎模式（auto 本地优先 / local 仅本地 / local-server 本地服务 / cloud 仅云端） */
function onAiModeChange(v: string) {
  settings.setAiConfig({ ...settings.ai, mode: v as AiMode });
  if (v === "local-server") {
    syncLocalServerConfig();
  } else {
    syncCloudConfig();
  }
}

function onCloudChange(key: "baseUrl" | "apiKey" | "embeddingModel" | "chatModel", v: string) {
  settings.setAiConfig({
    ...settings.ai,
    cloud: { ...settings.ai.cloud, [key]: v },
  });
  syncCloudConfig();
}

const testing = ref(false);
/** 本地服务连接测试 */
const localServerTesting = ref(false);
async function testCloud() {
  if (testing.value) return;
  testing.value = true;
  try {
    const provider = new CloudProvider(settings.ai.cloud);
    await provider.chat([{ role: "user", content: "ping" }]);
    message.success(t("settings.aiTestOk"));
  } catch (e: any) {
    message.error(t("settings.aiTestFail", { err: String(e) }));
  } finally {
    testing.value = false;
  }
}

/** 本地服务配置变更 */
function onLocalServerChange(key: "baseUrl" | "chatModel" | "embeddingModel", v: string) {
  settings.setAiConfig({
    ...settings.ai,
    localServer: { ...settings.ai.localServer, [key]: v },
  });
  syncLocalServerConfig();
}

/** 测试本地服务连接 */
async function testLocalServer() {
  if (localServerTesting.value) return;
  localServerTesting.value = true;
  try {
    const provider = new CloudProvider({
      baseUrl: settings.ai.localServer.baseUrl,
      apiKey: "",
      embeddingModel: settings.ai.localServer.embeddingModel,
      chatModel: settings.ai.localServer.chatModel,
    });
    await provider.chat([{ role: "user", content: "ping" }]);
    message.success(t("settings.aiTestOk"));
  } catch (e: any) {
    message.error(t("settings.aiTestFail", { err: String(e) }));
  } finally {
    localServerTesting.value = false;
  }
}

const chatStatusText = computed(() => {
  switch (chatStatus.value) {
    case "ready": return t("settings.aiLocalChatReady");
    case "downloading": return t("settings.aiLocalChatDownloading");
    default: return t("settings.aiLocalChatNotReady");
  }
});

async function refreshChatStatus() {
  chatStatus.value = await localChatModelStatus();
  try {
    chatSize.value = await localChatModelSize();
  } catch {
    chatSize.value = 0;
  }
}

async function downloadChat() {
  if (chatBusy.value) return;
  chatBusy.value = true;
  chatProgress.value = null;
  chatStatus.value = "downloading";
  try {
    await downloadLocalChatModel((p) => {
      chatProgress.value = p;
      chatStatus.value = "downloading";
    });
    message.success(t("settings.aiLocalChatDownloaded"));
    chatProgress.value = null;
    await refreshChatStatus();
  } catch (e: any) {
    chatStatus.value = "unavailable";
    chatProgress.value = null;
    message.error(t("settings.aiLocalChatDownloadFail", { err: e || "" }));
  } finally {
    chatBusy.value = false;
  }
}

async function deleteChat() {
  try {
    const n = await deleteLocalChatModel();
    chatSize.value = 0;
    chatStatus.value = "unavailable";
    message.success(t("settings.aiLocalChatDeleted", { n }));
  } catch (e: any) {
    message.error(t("settings.aiLocalChatDeleteFail", { err: String(e) }));
  }
}

const RULES: { key: string; labelKey: string; options: string[] }[] = [
  { key: "docx", labelKey: "settings.watcherRuleDocx", options: ["pdf", "txt", "html", "md"] },
  { key: "xlsx", labelKey: "settings.watcherRuleXlsx", options: ["pdf", "csv"] },
  { key: "pptx", labelKey: "settings.watcherRulePptx", options: ["pdf", "txt"] },
  { key: "epub", labelKey: "settings.watcherRuleEpub", options: ["pdf", "txt", "html", "md"] },
  { key: "txt", labelKey: "settings.watcherRuleTxt", options: ["pdf"] },
  { key: "md", labelKey: "settings.watcherRuleMd", options: ["pdf"] },
  { key: "html", labelKey: "settings.watcherRuleHtml", options: ["pdf"] },
  { key: "images", labelKey: "settings.watcherRuleImages", options: ["pdf"] },
];

const IMAGE_EXTS = ["png", "jpg", "jpeg", "bmp", "gif", "webp"];

const effectiveTargets = computed(() => {
  const t: Record<string, string> = {};
  for (const rule of RULES) {
    t[rule.key] = settings.watcher.targets[rule.key] ?? rule.options[0];
  }
  return t;
});

function buildTargets(): Record<string, string> {
  const t: Record<string, string> = {};
  for (const rule of RULES) {
    const v = effectiveTargets.value[rule.key];
    if (rule.key === "images") {
      for (const ext of IMAGE_EXTS) t[ext] = v;
    } else {
      t[rule.key] = v;
    }
  }
  return t;
}

async function applyWatcher(): Promise<boolean> {
  if (watcherBusy.value) return false;
  watcherBusy.value = true;
  try {
    await watcherStart(settings.watcher.folder, buildTargets());
    return true;
  } catch (e) {
    message.error(t("settings.watcherStartFail", { err: String(e) }));
    return false;
  } finally {
    watcherBusy.value = false;
  }
}

async function restartWatcherIfNeeded() {
  if (settings.watcher.enabled && settings.watcher.folder) {
    await applyWatcher();
  }
}

async function onWatcherToggle(v: boolean) {
  if (watcherBusy.value) return;
  if (v) {
    if (!settings.watcher.folder) {
      message.warning(t("settings.watcherNeedFolder"));
      return;
    }
    if (await applyWatcher()) {
      settings.setWatcher({ ...settings.watcher, enabled: true });
      message.success(t("settings.watcherEnabled"));
    }
  } else {
    watcherBusy.value = true;
    try {
      await watcherStop();
      settings.setWatcher({ ...settings.watcher, enabled: false });
      message.info(t("settings.watcherDisabled"));
    } catch (e) {
      message.error(String(e));
    } finally {
      watcherBusy.value = false;
    }
  }
}

async function chooseWatcherFolder() {
  const d = await openDialog({ directory: true, title: t("settings.watcherTitle") });
  if (d) {
    settings.setWatcher({ ...settings.watcher, folder: String(d) });
    await restartWatcherIfNeeded();
  }
}

async function clearWatcherFolder() {
  if (settings.watcher.enabled) {
    try { await watcherStop(); } catch { /* 忽略 */ }
  }
  settings.setWatcher({ ...settings.watcher, folder: "", enabled: false });
  message.info(t("settings.watcherCleared"));
}

async function onRuleChange(key: string, value: string) {
  settings.setWatcher({
    ...settings.watcher,
    targets: { ...settings.watcher.targets, [key]: value },
  });
  await restartWatcherIfNeeded();
}

const localeOptions = [
  { label: t("settings.followSystem"), value: "system" },
  { label: "简体中文", value: "zh-CN" },
  { label: "English", value: "en-US" },
  { label: "日本語", value: "ja-JP" },
  { label: "한국어", value: "ko-KR" },
];

async function chooseDir() {
  const d = await openDialog({ directory: true, title: t("settings.outDir") });
  if (d) {
    settings.setDefaultOutDir(String(d));
    message.success(t("settings.msgOutDirSet"));
  }
}

function clearDir() {
  settings.clearDefaultOutDir();
  message.info(t("settings.msgOutDirCleared"));
}

function onLocaleChange(v: string) {
  settings.setLocale(v as AppLocale);
}

/** 跟随系统开关：开→system，关→light */
function onThemeFollowToggle(v: boolean) {
  settings.setTheme(v ? "system" : "light");
}

/** 点击主题卡片直接切换 */
function onThemeCardClick(v: AppTheme) {
  settings.setTheme(v);
}

async function onAutostartChange(v: boolean) {
  try {
    if (v) {
      await enable();
      message.success(t("settings.msgAutostartOn"));
    } else {
      await disable();
      message.info(t("settings.msgAutostartOff"));
    }
    autostart.value = v;
  } catch (e) {
    autostart.value = !v;
    message.error(String(e));
  }
}

onMounted(async () => {
  try { autostart.value = await isEnabled(); } catch { /* 忽略 */ }
  if (settings.watcher.enabled && settings.watcher.folder) {
    await applyWatcher();
  }
  syncCloudConfig();
  syncLocalChatModel();
  localStatus.value = await localEngineStatus();
  await refreshChatStatus();
});
</script>

<style scoped>
/* ===== 面板头部（与其他功能面板对齐） ===== */
.settings-panel {
  max-width: 700px;
  margin: 0 auto;
}
.panel-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 22px;
}
.panel-head h2 {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
  color: var(--text-main);
}

/* ===== 设置卡片 ===== */
.setting-card {
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px 22px;
  margin-bottom: 20px;
  transition: box-shadow 0.15s;
}
.setting-card:hover {
  box-shadow: 0 1px 4px var(--shadow);
}

/* 卡片行：标题左侧 + 内容右侧 */
.card-line {
  display: flex;
  align-items: flex-start;
  gap: 20px;
}
.card-label {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 120px;
  flex-shrink: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main);
  padding-top: 5px;
}
.card-icon {
  flex-shrink: 0;
  color: var(--accent);
}
.card-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 控件行 */
.ctrl-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.ctrl-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.col-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
}
.ctrl-value {
  flex: 1;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  padding: 8px 12px;
  font-size: 13px;
  color: var(--text-body);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: var(--bg-input);
}
.ctrl-value.empty {
  color: var(--text-muted);
}
.card-hint {
  margin: 0;
  font-size: 11px;
  color: var(--text-muted);
}

/* ===== 主题预览卡片 ===== */
.theme-sys-label {
  font-size: 13px;
  color: var(--text-body);
}
.theme-picker {
  display: flex;
  gap: 14px;
}
.theme-card {
  flex: 1;
  background: var(--bg-panel);
  border: 1.5px solid var(--border);
  border-radius: 10px;
  padding: 14px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  position: relative;
}
.theme-card:hover {
  border-color: var(--accent);
  box-shadow: 0 1px 4px var(--shadow);
}
.theme-card.active {
  border-color: var(--accent);
  box-shadow: 0 0 0 1.5px var(--accent);
  background: var(--accent-soft);
}
.theme-preview {
  width: 100%;
  border-radius: 8px;
  padding: 12px 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.theme-light-preview {
  background: #f8f9fa;
  border: 1px solid #e9ecef;
}
.theme-dark-preview {
  background: #2d2d2d;
  border: 1px solid #404040;
}
.kb-row {
  display: flex;
  gap: 4px;
}
.kb-row span {
  flex: 1;
  height: 8px;
  border-radius: 2px;
}
.theme-light-preview .kb-row span {
  background: #dee2e6;
}
.theme-light-preview .kb-row span.wide {
  background: #adb5bd;
}
.theme-dark-preview .kb-row span {
  background: #555;
}
.theme-dark-preview .kb-row span.wide {
  background: #777;
}
.theme-check {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--accent);
  color: #fff;
}
.theme-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main);
}

/* ===== 标签页 ===== */
.settings-tabs {
  margin-top: 4px;
}

/* ===== 迷你按钮 ===== */
.mini-btn {
  border: none;
  border-radius: 8px;
  padding: 8px 16px;
  font-size: 13px;
  cursor: pointer;
  flex-shrink: 0;
  transition: opacity 0.15s, color 0.15s, background 0.15s;
}
.mini-btn.primary {
  background: var(--cta-bg);
  color: var(--cta-text);
}
.mini-btn.primary:hover {
  opacity: 0.85;
}
.mini-btn.ghost {
  background: var(--bg-tag);
  color: var(--text-sub);
}
.mini-btn.ghost:hover {
  color: var(--red);
  background: var(--red-soft);
}

/* ===== 本地模型状态标签 ===== */
.local-status {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 12px;
  border-radius: 12px;
}
.local-status.ready {
  background: var(--green-soft);
  color: var(--green);
}
.local-status.loading,
.local-status.downloading {
  background: var(--orange-soft);
  color: var(--orange);
}
.local-status.unavailable {
  background: var(--bg-tag);
  color: var(--text-muted);
}
.size-badge {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 12px;
  border-radius: 12px;
  background: var(--bg-tag);
  color: var(--text-muted);
}

/* ===== 下载进度 ===== */
.chat-progress {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.progress-file {
  font-size: 11px;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ===== 文件夹监控规则 ===== */
.watcher-rules {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 2px;
}
.rule-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.rule-label {
  width: 110px;
  flex-shrink: 0;
  font-size: 13px;
  color: var(--text-body);
}
.rule-row .n-select {
  flex: 1;
}

/* ===== naive-ui 组件样式覆盖 ===== */
:deep(.n-tabs .n-tabs-tab) {
  font-weight: 500;
}
:deep(.n-tabs .n-tabs-tab--active) {
  font-weight: 600;
}
:deep(.n-select .n-base-selection) {
  border-radius: 8px;
}
:deep(.n-input .n-input-wrapper) {
  border-radius: 8px;
}
:deep(.n-radio-group .n-radio-button) {
  border-radius: 6px;
}
/* AI 引擎模式单选按钮间距加宽 */
/* AI 引擎模式三卡片布局 */
.ai-mode-picker {
  display: flex;
  gap: 10px;
}
.ai-mode-picker .theme-card {
  padding: 16px 10px;
  gap: 10px;
}
.mode-icon-wrap {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tag);
  color: var(--accent);
}
.ai-mode-picker .theme-card.active .mode-icon-wrap {
  background: var(--accent-soft);
}
:deep(.n-button) {
  border-radius: 8px;
}
:deep(.n-switch .n-switch__button) {
  border-radius: 50%;
}
</style>