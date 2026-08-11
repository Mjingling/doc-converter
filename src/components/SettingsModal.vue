<template>
  <n-drawer v-model:show="show" :width="460" placement="right">
    <n-drawer-content :title="t('settings.title')" closable>
      <n-tabs v-model:value="tab" type="segment" size="small" class="settings-tabs">
        <!-- 通用设置 -->
        <n-tab-pane name="general" :tab="t('settings.tabGeneral')">
          <!-- 默认输出目录 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.outDir") }}</div>
            <div class="setting-row">
              <div class="setting-value" :class="{ empty: !settings.defaultOutDir }" :title="settings.defaultOutDir">
                {{ settings.defaultOutDir || t("settings.notSet") }}
              </div>
              <button class="mini-btn primary" @click="chooseDir">{{ t("settings.choose") }}</button>
              <button v-if="settings.defaultOutDir" class="mini-btn ghost" @click="clearDir">{{ t("settings.clear") }}</button>
            </div>
            <p class="setting-hint">{{ t("settings.hint") }}</p>
          </div>

          <!-- 语言 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.language") }}</div>
            <NSelect :value="settings.locale" :options="localeOptions" size="small" @update:value="onLocaleChange" />
          </div>

          <!-- 主题 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.theme") }}</div>
            <NRadioGroup :value="settings.theme" size="small" @update:value="onThemeChange">
              <NRadioButton value="system">{{ t("settings.followSystem") }}</NRadioButton>
              <NRadioButton value="light">{{ t("settings.light") }}</NRadioButton>
              <NRadioButton value="dark">{{ t("settings.dark") }}</NRadioButton>
            </NRadioGroup>
          </div>

          <!-- 开机启动 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.autostart") }}</div>
            <div class="setting-row">
              <NSwitch :value="autostart" size="small" @update:value="onAutostartChange" />
            </div>
            <p class="setting-hint">{{ t("settings.autostartHint") }}</p>
          </div>
        </n-tab-pane>

        <!-- 高级设置 -->
        <n-tab-pane name="advanced" :tab="t('settings.tabAdvanced')">
          <!-- 文件夹监控 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.watcherTitle") }}</div>
            <div class="setting-row">
              <NSwitch :value="settings.watcher.enabled" size="small" :disabled="watcherBusy" @update:value="onWatcherToggle" />
              <p class="setting-hint">{{ t("settings.watcherHint") }}</p>
            </div>
            <div class="setting-row">
              <div class="setting-value" :class="{ empty: !settings.watcher.folder }" :title="settings.watcher.folder">
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
            <p class="setting-hint">{{ t("settings.watcherNeedLo") }}</p>
            <p class="setting-hint">{{ t("settings.watcherOutDirHint") }}</p>
          </div>
        </n-tab-pane>

        <!-- AI 能力 -->
        <n-tab-pane name="ai" :tab="t('settings.tabAi')">
          <!-- 引擎模式 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.aiMode") }}</div>
            <NRadioGroup :value="settings.ai.mode" size="small" @update:value="onAiModeChange">
              <NRadioButton value="auto">{{ t("settings.aiModeAuto") }}</NRadioButton>
              <NRadioButton value="local">{{ t("settings.aiModeLocal") }}</NRadioButton>
              <NRadioButton value="cloud">{{ t("settings.aiModeCloud") }}</NRadioButton>
            </NRadioGroup>
            <p class="setting-hint">{{ t("settings.aiModeHint") }}</p>
          </div>

          <!-- 云端 API 配置（local 模式隐藏） -->
          <div v-if="settings.ai.mode !== 'local'" class="setting-group">
            <div class="setting-label">{{ t("settings.aiCloudSection") }}</div>
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
            <div class="setting-row">
              <div class="setting-col">
                <div class="setting-label small">{{ t("settings.aiEmbeddingModel") }}</div>
                <NInput
                  size="small"
                  :value="settings.ai.cloud.embeddingModel"
                  @update:value="(v: string) => onCloudChange('embeddingModel', v)"
                />
              </div>
              <div class="setting-col">
                <div class="setting-label small">{{ t("settings.aiChatModel") }}</div>
                <NInput
                  size="small"
                  :value="settings.ai.cloud.chatModel"
                  @update:value="(v: string) => onCloudChange('chatModel', v)"
                />
              </div>
            </div>
            <p class="setting-hint">{{ t("settings.aiCloudHint") }}</p>
            <div class="setting-row">
              <NButton
                size="small"
                :loading="testing"
                :disabled="!settings.ai.cloud.baseUrl || !settings.ai.cloud.apiKey"
                @click="testCloud"
              >{{ t("settings.aiTest") }}</NButton>
            </div>
          </div>

          <!-- 本地模型状态 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.aiLocalSection") }}</div>
            <div class="setting-row">
              <span class="setting-value">{{ t("settings.aiLocalEmbedding") }}</span>
              <span class="local-status" :class="localStatus">{{ localStatusText }}</span>
            </div>
            <p class="setting-hint">{{ t("settings.aiLocalHint") }}</p>
          </div>

          <!-- 本地生成式模型（chat）管理 -->
          <div class="setting-group">
            <div class="setting-label">{{ t("settings.aiLocalChatSection") }}</div>
            <NInput
              size="small"
              :value="settings.ai.localChatModelId"
              :placeholder="t('settings.aiLocalChatModelPlaceholder')"
              :disabled="chatBusy || chatStatus === 'downloading'"
              @update:value="onChatModelIdChange"
            />
            <div class="setting-row">
              <span class="setting-value">{{ t("settings.aiLocalChatStatus") }}</span>
              <span v-if="chatSize > 0" class="size-badge">{{ formatBytes(chatSize) }}</span>
              <span class="local-status" :class="chatStatus">{{ chatStatusText }}</span>
            </div>
            <div v-if="chatProgress" class="chat-progress">
              <NProgress :percentage="chatProgress.percent" :indicator-placement="'inside'" processing />
              <span class="progress-file">{{ chatProgress.file }}（{{ formatBytes(chatProgress.loaded) }} / {{ formatBytes(chatProgress.total) }}）</span>
            </div>
            <div class="setting-row">
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
            <p class="setting-hint">{{ t("settings.aiLocalChatHint") }}</p>
          </div>
        </n-tab-pane>
      </n-tabs>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NButton, NDrawer, NDrawerContent, NInput, NPopconfirm, NProgress, NRadioButton, NRadioGroup, NSelect, NSwitch, NTabPane, NTabs, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { watcherStart, watcherStop } from "../api";
import { useSettingsStore } from "../stores/settings";
import type { AiMode, AppLocale, AppTheme } from "../stores/settings";
import { localEngineStatus, syncCloudConfig, syncLocalChatModel, localChatModelStatus, downloadLocalChatModel, deleteLocalChatModel, localChatModelSize, formatBytes, CloudProvider } from "../ai";
import type { ChatModelProgress, ChatModelState } from "../ai";

const { t } = useI18n();
const show = defineModel<boolean>("show", { default: false });
const settings = useSettingsStore();
const message = useMessage();

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

/** 切换 AI 引擎模式（auto 本地优先 / local 仅本地 / cloud 仅云端） */
function onAiModeChange(v: string) {
  settings.setAiConfig({ ...settings.ai, mode: v as AiMode });
  syncCloudConfig();
}

/** 更新云端 API 配置项（baseUrl / apiKey / embeddingModel / chatModel） */
function onCloudChange(key: "baseUrl" | "apiKey" | "embeddingModel" | "chatModel", v: string) {
  settings.setAiConfig({
    ...settings.ai,
    cloud: { ...settings.ai.cloud, [key]: v },
  });
  syncCloudConfig();
}

/** AI 云端配置测试连接 */
const testing = ref(false);
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

/** 本地生成式模型状态文案 */
const chatStatusText = computed(() => {
  switch (chatStatus.value) {
    case "ready": return t("settings.aiLocalChatReady");
    case "downloading": return t("settings.aiLocalChatDownloading");
    default: return t("settings.aiLocalChatNotReady");
  }
});

/** 修改本地 chat 模型 ID：保存设置并重置 provider 缓存 */
function onChatModelIdChange(v: string) {
  settings.setAiConfig({ ...settings.ai, localChatModelId: v });
  syncLocalChatModel();
  refreshChatStatus();
}

/** 刷新本地 chat 模型状态与磁盘占用 */
async function refreshChatStatus() {
  chatStatus.value = await localChatModelStatus();
  try {
    chatSize.value = await localChatModelSize();
  } catch {
    chatSize.value = 0;
  }
}

/** 下载本地 chat 模型（首次下载较大，进度条实时反馈） */
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
    message.error(t("settings.aiLocalChatDownloadFail", { err: String(e || "") }));
  } finally {
    chatBusy.value = false;
  }
}

/** 删除本地 chat 模型缓存（释放磁盘；重启后彻底生效） */
async function deleteChat() {
  try {
    const n = await deleteLocalChatModel();
    chatSize.value = 0;
    chatStatus.value = "unavailable";
    message.success(t("settings.aiLocalChatDeleted", { n }));
  } catch (e: any) {
    message.error(t("settings.aiLocalChatDeleteFail", { err: String(e || "") }));
  }
}

/** 可监控的输入格式规则：图片规则展开为全部图片扩展名 */
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

/** 各规则当前目标格式（未自定义时用每行第一个选项） */
const effectiveTargets = computed(() => {
  const t: Record<string, string> = {};
  for (const rule of RULES) {
    t[rule.key] = settings.watcher.targets[rule.key] ?? rule.options[0];
  }
  return t;
});

/** 组装传给后端的「扩展名 → 目标扩展名」映射（images 规则展开为图片扩展名） */
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

/** 启动（或重启）文件夹监控；失败时返回 false 并提示 */
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

/** 已启用时用新配置重启监控（目录 / 规则变更后调用） */
async function restartWatcherIfNeeded() {
  if (settings.watcher.enabled && settings.watcher.folder) {
    await applyWatcher();
  }
}

/** 启用 / 禁用监控 */
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

/** 选择监控目录（已启用时自动重启监控） */
async function chooseWatcherFolder() {
  const d = await openDialog({ directory: true, title: t("settings.watcherTitle") });
  if (d) {
    settings.setWatcher({ ...settings.watcher, folder: String(d) });
    await restartWatcherIfNeeded();
  }
}

/** 清除监控目录并停止监控 */
async function clearWatcherFolder() {
  if (settings.watcher.enabled) {
    try {
      await watcherStop();
    } catch {
      /* 停止失败时仅保持本地配置 */
    }
  }
  settings.setWatcher({ ...settings.watcher, folder: "", enabled: false });
  message.info(t("settings.watcherCleared"));
}

/** 修改格式规则（已启用时自动重启监控使其生效） */
async function onRuleChange(key: string, value: string) {
  settings.setWatcher({
    ...settings.watcher,
    targets: { ...settings.watcher.targets, [key]: value },
  });
  await restartWatcherIfNeeded();
}

/** 语言选项：语言名使用各自本地名称（无需翻译） */
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

function onThemeChange(v: string) {
  settings.setTheme(v as AppTheme);
}

/** 切换开机启动；失败时回滚开关状态 */
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
  try {
    autostart.value = await isEnabled();
  } catch {
    /* 读取失败时保持默认关闭 */
  }
  // 恢复文件夹监控：上次已启用且目录仍有效时自动重新监听
  if (settings.watcher.enabled && settings.watcher.folder) {
    await applyWatcher();
  }
  // 同步云端 AI 配置到 provider，并刷新本地模型状态
  syncCloudConfig();
  syncLocalChatModel();
  localStatus.value = await localEngineStatus();
  await refreshChatStatus();
});
</script>

<style scoped>
.setting-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.settings-tabs {
  margin-top: 4px;
}
/* 抽屉标题栏与主界面标题栏同高同风格（40px） */
:deep(.n-drawer-header) {
  height: 40px;
  padding: 0 16px;
  box-sizing: border-box;
}
:deep(.n-drawer-header__main) {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-sub);
}
.setting-group + .setting-group {
  margin-top: 18px;
}
.setting-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-body);
}
.setting-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.setting-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.setting-label.small {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
}
.local-status {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 12px;
  border-radius: 12px;
}
.local-status.ready { background: #e6ffed; color: #1a7f37; }
.local-status.loading { background: #fff8c5; color: #9a6700; }
.local-status.unavailable { background: var(--bg-tag); color: var(--text-muted); }
.local-status.downloading { background: #fff8c5; color: #9a6700; }
.size-badge {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 12px;
  border-radius: 12px;
  background: var(--bg-tag);
  color: var(--text-muted);
}
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
.setting-value {
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
.setting-value.empty {
  color: var(--text-muted);
}
.mini-btn {
  border: none;
  border-radius: 8px;
  padding: 8px 16px;
  font-size: 13px;
  cursor: pointer;
  flex-shrink: 0;
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
.setting-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}
.watcher-rules {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 4px;
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
</style>
