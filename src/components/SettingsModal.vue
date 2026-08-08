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
      </n-tabs>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NDrawer, NDrawerContent, NRadioButton, NRadioGroup, NSelect, NSwitch, NTabPane, NTabs, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { watcherStart, watcherStop } from "../api";
import { useSettingsStore } from "../stores/settings";
import type { AppLocale, AppTheme } from "../stores/settings";

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
