<template>
  <div class="settings-panel">
    <!-- 左侧分类导航（个人资料 / 常规 / 模式配置 / 高级） -->
    <aside class="settings-nav">
      <button
        v-for="nav in SETTING_NAVS"
        :key="nav.id"
        class="settings-nav-item"
        :class="{ active: tab === nav.id }"
        @click="tab = nav.id"
      >
        <NIcon :component="nav.icon" :size="16" />
        <span>{{ t(nav.labelKey) }}</span>
      </button>
    </aside>

    <!-- 右侧内容区 -->
    <div class="settings-content">
      <!-- 个人资料：头像 / 统计 / 版本 / 任务记录 -->
      <template v-if="tab === 'profile'">
        <!-- 资料卡 -->
        <div class="setting-card">
          <div class="profile-head">
            <div class="profile-avatar">{{ t("app.name").charAt(0) }}</div>
            <div class="profile-meta">
              <div class="profile-name-row">
                <span class="profile-name">{{ t("app.name") }}</span>
                <span class="profile-tag">{{ t("settings.profileTag") }}</span>
              </div>
              <div class="profile-sub">v{{ version || "0.0.0" }}</div>
            </div>
          </div>
          <div class="stat-grid">
            <div v-for="s in usageStats" :key="s.labelKey" class="stat-card">
              <div class="stat-num">{{ s.value }}</div>
              <div class="stat-label">{{ t(s.labelKey) }}</div>
            </div>
          </div>
        </div>

        <!-- 当前版本 + 检查更新 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="CubeOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.currentVersion") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <span class="size-badge">{{ version ? `v${version}` : "v0.0.0" }}</span>
                <button class="mini-btn primary" @click="showUpdate = true">{{ t("update.checkBtn") }}</button>
              </div>
              <p class="card-hint">{{ t("settings.aboutHint") }}</p>
            </div>
          </div>
        </div>

        <!-- 任务记录 + 清除 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="TimeOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.historyTitle") }}</span>
            </div>
            <div class="card-content">
              <p class="card-hint">{{ t("settings.clearHistoryHint") }}</p>
              <NPopconfirm :positive-text="t('settings.ok')" :negative-text="t('settings.cancel')" @positive-click="clearHistory">
                <template #trigger>
                  <NButton size="small" type="error" class="clear-history-btn">
                    <template #icon>
                      <NIcon :component="TrashOutline" />
                    </template>
                    {{ t("settings.clearHistoryBtn") }}
                  </NButton>
                </template>
                {{ t("settings.clearHistoryConfirm") }}
              </NPopconfirm>
            </div>
          </div>
        </div>
      </template>

      <!-- 常规 -->
      <template v-else-if="tab === 'general'">
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
                <NSwitch :value="autostart" :disabled="autostartBusy" size="small" @update:value="onAutostartChange" />
              </div>
              <p class="card-hint">{{ t("settings.autostartHint") }}</p>
            </div>
          </div>
        </div>

        <!-- 任务完成通知 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="NotificationsOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.notifyOnComplete") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="settings.notifyOnComplete" size="small" @update:value="onNotifyToggle" />
              </div>
              <p class="card-hint">{{ t("settings.notifyOnCompleteHint") }}</p>
            </div>
          </div>
        </div>

      </template>

      <!-- 输出目录 -->
      <template v-else-if="tab === 'outdir'">
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

        <!-- 完成后自动打开输出目录 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="OpenOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.autoOpen") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="settings.outdir.autoOpen" size="small" @update:value="onAutoOpenToggle" />
              </div>
              <p class="card-hint">{{ t("settings.autoOpenHint") }}</p>
            </div>
          </div>
        </div>

        <!-- 同名文件策略 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="CopyOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.conflictPolicy") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSelect
                  size="small"
                  :value="settings.outdir.conflict"
                  :options="conflictOptions"
                  @update:value="onConflictChange"
                />
              </div>
              <p class="card-hint">{{ t("settings.conflictHint") }}</p>
            </div>
          </div>
        </div>
      </template>

      <!-- 快捷键 -->
      <template v-else-if="tab === 'shortcut'">
        <!-- 唤起主窗口 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="KeypadOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.shortcutTitle") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="shortcutEnabled" size="small" :disabled="recording !== null" @update:value="(v) => onShortcutToggle('main', v)" />
                <span v-if="recording === 'main'" class="shortcut-recording">{{ t("settings.shortcutRecording") }}</span>
                <span v-else-if="shortcutEnabled" class="shortcut-badge">{{ formatShortcut(settings.globalShortcut) }}</span>
                <button v-if="shortcutEnabled" class="mini-btn primary" :disabled="recording !== null" @click="startRecording('main')">
                  {{ recording === 'main' ? t("settings.shortcutRecording") : t("settings.shortcutModify") }}
                </button>
                <button v-if="shortcutEnabled && settings.globalShortcut !== DEFAULT_GLOBAL_SHORTCUT" class="mini-btn ghost" :disabled="recording !== null" @click="resetShortcut('main')">
                  {{ t("settings.shortcutReset") }}
                </button>
              </div>
              <p class="card-hint">{{ t("settings.shortcutHint") }}</p>
            </div>
          </div>
        </div>

        <!-- 唤起 AI 助手 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="ChatbubbleEllipsesOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.assistantShortcutTitle") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="assistantShortcutEnabled" size="small" :disabled="recording !== null" @update:value="(v) => onShortcutToggle('assistant', v)" />
                <span v-if="recording === 'assistant'" class="shortcut-recording">{{ t("settings.shortcutRecording") }}</span>
                <span v-else-if="assistantShortcutEnabled" class="shortcut-badge">{{ formatShortcut(settings.assistantShortcut) }}</span>
                <button v-if="assistantShortcutEnabled" class="mini-btn primary" :disabled="recording !== null" @click="startRecording('assistant')">
                  {{ recording === 'assistant' ? t("settings.shortcutRecording") : t("settings.shortcutModify") }}
                </button>
              </div>
              <p class="card-hint">{{ t("settings.assistantShortcutHint") }}</p>
            </div>
          </div>
        </div>
      </template>

      <!-- 桌面宠物 -->
      <template v-else-if="tab === 'pet'">
        <!-- 显示开关 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="HappyOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.pet") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="settings.pet.enabled" size="small" @update:value="onPetToggle" />
              </div>
              <p class="card-hint">{{ t("settings.petHint") }}</p>
            </div>
          </div>
        </div>

        <!-- 大小 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="ResizeOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.petSize") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row pet-size-row">
                <NSlider :value="settings.pet.size" :min="0.6" :max="1.5" :step="0.1" :format-tooltip="(v) => `${Math.round(v * 100)}%`" style="max-width: 200px" @update:value="onPetSizeChange" />
                <span class="pet-size-label">{{ Math.round(settings.pet.size * 100) }}%</span>
              </div>
              <p class="card-hint">{{ t("settings.petSizeHint") }}</p>
            </div>
          </div>
        </div>

        <!-- 活跃度 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="PulseOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.petActivity") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSelect size="small" :value="settings.pet.activity" :options="petActivityOptions" @update:value="onPetActivityChange" />
              </div>
              <p class="card-hint">{{ t("settings.petActivityHint") }}</p>
            </div>
          </div>
        </div>

        <!-- 昼夜节律 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="MoonOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.petCircadian") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <NSwitch :value="settings.pet.circadian" size="small" @update:value="onCircadianToggle" />
              </div>
              <p class="card-hint">{{ t("settings.petCircadianHint") }}</p>
            </div>
          </div>
        </div>
      </template>

      <!-- 高级 -->
      <template v-else-if="tab === 'advanced'">
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

        <!-- 诊断排障：日志目录 + 开发者工具 -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="BugOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.diagLogTitle") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row">
                <button class="mini-btn primary" @click="openLogDir">{{ t("settings.diagLogOpen") }}</button>
                <button class="mini-btn ghost" @click="onOpenDevtools">{{ t("settings.devtoolsOpen") }}</button>
              </div>
              <p class="card-hint">{{ t("settings.diagLogHint") }}</p>
              <p class="card-hint">{{ t("settings.devtoolsHint") }}</p>
            </div>
          </div>
        </div>
      </template>

      <!-- 模式配置 -->
      <template v-else>
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
              <div class="ctrl-row preset-row">
                <span class="col-label">{{ t("settings.aiProviderPreset") }}</span>
                <button
                  v-for="preset in CLOUD_AI_PRESETS"
                  :key="preset.id"
                  class="mini-btn preset-btn"
                  :class="{ active: activePresetId === preset.id }"
                  @click="applyCloudPreset(preset)"
                >{{ t(preset.labelKey) }}</button>
                <span v-if="activePresetId === 'custom'" class="preset-custom">{{ t("settings.aiPresetCustom") }}</span>
              </div>
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
              <p class="card-hint">{{ t("settings.aiPresetHint") }}</p>
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

        <!-- 网页搜索（AI 助手实时信息查询） -->
        <div class="setting-card">
          <div class="card-line">
            <div class="card-label">
              <NIcon :component="SearchOutline" :size="16" class="card-icon" />
              <span>{{ t("settings.searchTitle") }}</span>
            </div>
            <div class="card-content">
              <div class="ctrl-row preset-row">
                <button
                  v-for="opt in SEARCH_PROVIDERS"
                  :key="opt.id"
                  class="mini-btn preset-btn"
                  :class="{ active: settings.ai.search.provider === opt.id }"
                  @click="onSearchProviderChange(opt.id)"
                >{{ t(opt.labelKey) }}</button>
              </div>
              <NInput
                v-if="settings.ai.search.provider === 'tavily'"
                size="small"
                type="password"
                show-password-on="click"
                :value="settings.ai.search.tavilyKey"
                :placeholder="t('settings.searchTavilyKeyPlaceholder')"
                @update:value="(v: string) => onSearchChange('tavilyKey', v)"
              />
              <p v-if="settings.ai.search.provider === 'zhipu'" class="card-hint">{{ t("settings.searchZhipuReuseHint") }}</p>
              <p v-if="settings.ai.search.provider === 'tavily'" class="card-hint">{{ t("settings.searchTavilyHint") }}</p>
              <p class="card-hint">{{ t("settings.searchHint") }}</p>
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
                <span>{{ t("settings.aiLocalEmbedding") }}</span>
                <span v-if="embedSize > 0" class="size-badge">{{ formatBytes(embedSize) }}</span>
                <span class="local-status" :class="embedStatus">{{ embedStatusText }}</span>
              </div>
              <div v-if="embedProgress" class="chat-progress">
                <NProgress :percentage="embedProgress.percent" :indicator-placement="'inside'" processing />
                <span class="progress-file">{{ embedProgress.file }}（{{ formatBytes(embedProgress.loaded) }} / {{ formatBytes(embedProgress.total) }}）</span>
              </div>
              <div class="ctrl-row">
                <NButton size="small" type="primary" ghost :loading="embedBusy" :disabled="embedStatus === 'downloading'" @click="downloadEmbed">
                  {{ t("settings.aiLocalEmbedDownload") }}
                </NButton>
                <NPopconfirm :positive-text="t('settings.ok')" :negative-text="t('settings.cancel')" @positive-click="deleteEmbed">
                  <template #trigger>
                    <NButton size="small" type="error" ghost :disabled="embedStatus === 'downloading' || embedBusy">
                      {{ t("settings.aiLocalEmbedDelete") }}
                    </NButton>
                  </template>
                  {{ t("settings.aiLocalEmbedDeleteConfirm") }}
                </NPopconfirm>
              </div>
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
      </template>
    </div>

    <UpdateModal v-model:show="showUpdate" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { NButton, NIcon, NInput, NPopconfirm, NProgress, NSelect, NSlider, NSwitch, useMessage } from "naive-ui";
import {
  BugOutline,
  CheckmarkOutline, ChatbubbleEllipsesOutline, CloudOutline, ColorPaletteOutline, ConstructOutline, CopyOutline, CubeOutline,
  DesktopOutline,
  EyeOutline, FolderOpenOutline, GlobeOutline, HappyOutline, KeypadOutline, MoonOutline, NotificationsOutline, OpenOutline, OptionsOutline, PersonOutline, PowerOutline, PulseOutline, ResizeOutline, SearchOutline, SparklesOutline, TimeOutline, TrashOutline,
} from "@vicons/ionicons5";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { appDataDir } from "@tauri-apps/api/path";
import { getVersion } from "@tauri-apps/api/app";
import { openDevtools, openPath, watcherStart, watcherStop } from "../api";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";
import { buildShortcutFromEvent, DEFAULT_GLOBAL_SHORTCUT, formatShortcut } from "../utils/shortcut";
import type { AiMode, AppLocale, AppTheme } from "../stores/settings";
import UpdateModal from "./UpdateModal.vue";
import { localEngineStatus, syncCloudConfig, syncLocalChatModel, localChatModelStatus, downloadLocalChatModel, deleteLocalChatModel, localChatModelSize, localEmbedModelStatus, downloadLocalEmbedModel, deleteLocalEmbedModel, localEmbedModelSize, formatBytes, CloudProvider, cloudDiag, formatDiag, syncLocalServerConfig, CLOUD_AI_PRESETS } from "../ai";
import type { ChatModelProgress, ChatModelState, CloudAiPreset } from "../ai";

const { t } = useI18n();
const settings = useSettingsStore();
const historyStore = useHistoryStore();
const message = useMessage();

/** 检查更新弹窗开关 */
const showUpdate = ref(false);
/** 当前应用版本（检查更新弹窗与关于卡片展示） */
const version = ref("");

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
// 用户是否手动切换过开机启动：切换后初始化读取不再覆盖开关状态
let autostartTouched = false;
// 系统操作在途标记：禁用开关，防止快速双击 enable/disable 并发交错导致注册表终态与 UI 相反
const autostartBusy = ref(false);

/** 当前页签：个人资料 / 常规 / 模式配置 / 高级 */
const tab = ref("profile");

/** 左侧分类导航（顺序即展示顺序） */
const SETTING_NAVS = [
  { id: "profile", labelKey: "settings.profile", icon: PersonOutline },
  { id: "general", labelKey: "settings.tabGeneral", icon: OptionsOutline },
  { id: "outdir", labelKey: "settings.outdirTab", icon: FolderOpenOutline },
  { id: "shortcut", labelKey: "settings.shortcutTab", icon: KeypadOutline },
  { id: "pet", labelKey: "settings.petTab", icon: HappyOutline },
  { id: "ai", labelKey: "settings.tabAi", icon: SparklesOutline },
  { id: "advanced", labelKey: "settings.tabAdvanced", icon: ConstructOutline },
] as const;

/** 有历史记录的日期集合（本地时区 yyyy-mm-dd，升序） */
const activeDates = computed(() => {
  const set = new Set<string>();
  for (const item of historyStore.items) {
    const d = new Date(item.time);
    set.add(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`);
  }
  return [...set].sort();
});

/** 个人资料页统计：当前连续天数 / 最长连续天数 / 累计活跃天数 / 任务总数 */
const usageStats = computed(() => {
  const dates = activeDates.value;
  const dateSet = new Set(dates);
  const fmt = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  // 当前连续：从今天往前数；今天无记录则从昨天起
  const today = new Date();
  const cursor = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  if (!dateSet.has(fmt(cursor))) cursor.setDate(cursor.getDate() - 1);
  let current = 0;
  while (dateSet.has(fmt(cursor))) {
    current++;
    cursor.setDate(cursor.getDate() - 1);
  }
  // 最长连续：相邻日期差 1 天则续段
  let longest = 0;
  let run = 0;
  let prevTime: number | null = null;
  for (const s of dates) {
    const [y, m, d] = s.split("-").map(Number);
    const t = new Date(y, m - 1, d).getTime();
    if (prevTime !== null && Math.round((t - prevTime) / 86400000) === 1) run++;
    else run = 1;
    longest = Math.max(longest, run);
    prevTime = t;
  }
  return [
    { value: current, labelKey: "settings.statStreak" },
    { value: longest, labelKey: "settings.statLongestStreak" },
    { value: dates.length, labelKey: "settings.statActiveDays" },
    { value: historyStore.items.length, labelKey: "settings.statTasks" },
  ];
});

/** 清除全部任务记录（二次确认后执行） */
async function clearHistory() {
  await historyStore.clear();
  message.success(t("settings.clearHistoryDone"));
}

/** 监控操作进行中（防止重复启停） */
const watcherBusy = ref(false);

/* ---------- 全局快捷键（唤起主窗口 / AI 助手两类） ---------- */
/** 快捷键类别：main 唤起主窗口 / assistant 唤起 AI 助手 */
type ShortcutKind = "main" | "assistant";
/** 是否在录制模式（值 = 正在录制的类别；null = 未录制） */
const recording = ref<ShortcutKind | null>(null);
/** 各类快捷键开关（空字符串 = 禁用） */
const shortcutEnabled = computed(() => settings.globalShortcut !== "");
const assistantShortcutEnabled = computed(() => settings.assistantShortcut !== "");
/** 禁用前记住的键位：重新启用时恢复 */
const lastShortcut = ref<Record<ShortcutKind, string>>({
  main: DEFAULT_GLOBAL_SHORTCUT,
  assistant: "",
});

/** 录制模式按键处理：组合键即保存，Esc 取消 */
function onRecordKey(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    cancelRecording();
    return;
  }
  const shortcut = buildShortcutFromEvent(e);
  const kind = recording.value;
  if (!shortcut || !kind) return;
  cancelRecording();
  void applyShortcut(kind, shortcut);
}

function startRecording(kind: ShortcutKind = "main") {
  recording.value = kind;
  window.addEventListener("keydown", onRecordKey, true);
}

function cancelRecording() {
  recording.value = null;
  window.removeEventListener("keydown", onRecordKey, true);
}

/** 应用并持久化快捷键（成功提示 / 注册失败提示，如与其他应用冲突） */
async function applyShortcut(kind: ShortcutKind, shortcut: string) {
  try {
    if (kind === "main") await settings.setGlobalShortcut(shortcut);
    else await settings.setAssistantShortcut(shortcut);
    if (shortcut) lastShortcut.value[kind] = shortcut;
    message.success(t("settings.shortcutSaved"));
  } catch (e) {
    message.error(t("settings.shortcutFail", { err: String(e) }), { duration: 5000 });
  }
}

/** 开关快捷键：开=恢复上次键位，关=禁用 */
function onShortcutToggle(kind: ShortcutKind, enabled: boolean) {
  const fallback = kind === "main" ? DEFAULT_GLOBAL_SHORTCUT : "";
  void applyShortcut(kind, enabled ? lastShortcut.value[kind] || fallback : "");
}

/** 恢复默认快捷键 */
function resetShortcut(kind: ShortcutKind = "main") {
  void applyShortcut(kind, kind === "main" ? DEFAULT_GLOBAL_SHORTCUT : "");
}

onUnmounted(cancelRecording);

/** 本地 AI 引擎状态：unavailable（未加载）/ loading / ready */
const localStatus = ref<"unavailable" | "loading" | "ready">("unavailable");

/** 本地 embedding 模型状态：unavailable（未下载）/ downloading / ready */
const embedStatus = ref<ChatModelState>("unavailable");
/** 本地 embedding 模型缓存大小（字节） */
const embedSize = ref(0);
/** 下载进行中（防止重复点击） */
const embedBusy = ref(false);
/** 下载进度（null = 未在下载） */
const embedProgress = ref<ChatModelProgress | null>(null);

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

/** embedding 模型状态文案 */
const embedStatusText = computed(() => {
  switch (embedStatus.value) {
    case "ready": return t("settings.aiLocalEmbedReady");
    case "downloading": return t("settings.aiLocalEmbedDownloading");
    default: return t("settings.aiLocalEmbedNotReady");
  }
});

/** 刷新本地 embedding 模型状态与磁盘占用 */
async function refreshEmbedStatus() {
  embedStatus.value = await localEmbedModelStatus();
  try {
    embedSize.value = await localEmbedModelSize();
  } catch {
    embedSize.value = 0;
  }
}

/** 下载本地 embedding 模型（首次约 100MB，进度条实时反馈） */
async function downloadEmbed() {
  if (embedBusy.value) return;
  embedBusy.value = true;
  embedProgress.value = null;
  embedStatus.value = "downloading";
  try {
    await downloadLocalEmbedModel((p) => {
      embedProgress.value = p;
      embedStatus.value = "downloading";
    });
    message.success(t("settings.aiLocalEmbedDownloaded"));
    embedProgress.value = null;
    await refreshEmbedStatus();
  } catch (e: any) {
    embedStatus.value = "unavailable";
    embedProgress.value = null;
    message.error(t("settings.aiLocalEmbedDownloadFail", { err: e || "" }));
  } finally {
    embedBusy.value = false;
  }
}

/** 删除本地 embedding 模型缓存（释放磁盘；重启后彻底生效） */
async function deleteEmbed() {
  try {
    const n = await deleteLocalEmbedModel();
    embedSize.value = 0;
    embedStatus.value = "unavailable";
    message.success(t("settings.aiLocalEmbedDeleted", { n }));
  } catch (e: any) {
    message.error(t("settings.aiLocalEmbedDeleteFail", { err: String(e) }));
  }
}

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

/* ---------- 网页搜索配置 ---------- */

/** 搜索提供商选项（id 与 settings.ai.search.provider 对应） */
const SEARCH_PROVIDERS = [
  { id: "off", labelKey: "settings.searchOff" },
  { id: "zhipu", labelKey: "settings.searchZhipu" },
  { id: "tavily", labelKey: "settings.searchTavily" },
] as const;

function onSearchProviderChange(id: "off" | "zhipu" | "tavily") {
  settings.setAiConfig({
    ...settings.ai,
    search: { ...settings.ai.search, provider: id },
  });
}

function onSearchChange(key: "tavilyKey", v: string) {
  settings.setAiConfig({
    ...settings.ai,
    search: { ...settings.ai.search, [key]: v },
  });
}

/** 当前云端配置命中的预设（用户改动任一字段即回落为自定义） */
const activePresetId = computed(() => {
  const c = settings.ai.cloud;
  return CLOUD_AI_PRESETS.find(
    (p) => p.baseUrl === c.baseUrl && p.chatModel === c.chatModel && p.embeddingModel === c.embeddingModel,
  )?.id ?? "custom";
});

/** 一键套用服务商预设（填充地址与模型，保留已填写的 API 密钥） */
function applyCloudPreset(preset: CloudAiPreset) {
  settings.setAiConfig({
    ...settings.ai,
    cloud: {
      ...settings.ai.cloud,
      baseUrl: preset.baseUrl,
      chatModel: preset.chatModel,
      embeddingModel: preset.embeddingModel,
    },
  });
  syncCloudConfig();
}

const testing = ref(false);
/** 本地服务连接测试 */
const localServerTesting = ref(false);

/** 连接失败后的通用处理：先报基础错误，再分阶段诊断并把卡点拼进提示（诊断可耗时数秒，不阻塞首条反馈） */
async function reportConnectFail(baseUrl: string, err: string) {
  message.error(t("settings.aiTestFail", { err }), { duration: 15000 });
  try {
    const diag = await cloudDiag(baseUrl);
    message.error(t("settings.aiTestFail", { err: `${err}\n${formatDiag(diag)}` }), { duration: 20000 });
  } catch {
    // 诊断本身失败（如非 Tauri 环境）：保留上面的基础错误
  }
}

async function testCloud() {
  if (testing.value) return;
  testing.value = true;
  try {
    const provider = new CloudProvider(settings.ai.cloud);
    await provider.chat([{ role: "user", content: "ping" }]);
    message.success(t("settings.aiTestOk"));
  } catch (e: any) {
    await reportConnectFail(settings.ai.cloud.baseUrl, String(e));
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
    await reportConnectFail(settings.ai.localServer.baseUrl, String(e));
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

/** 打开应用数据目录（诊断日志所在处，如 pet-diag.log） */
async function openLogDir() {
  try {
    const dir = await appDataDir();
    await openPath(dir);
    message.success(t("settings.diagLogOpen"));
  } catch (e) {
    console.error("[openLogDir] error:", e);
    message.error(t("settings.openLogDirFail", { err: String(e) }));
  }
}

/** 打开主窗口开发者工具（排障） */
async function onOpenDevtools() {
  try {
    await openDevtools();
    message.success(t("settings.devtoolsOpen"));
  } catch (e) {
    console.error("[onOpenDevtools] error:", e);
    message.error(t("settings.openDevtoolsFail", { err: String(e) }));
  }
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

/** 桌面宠物开关：持久化并即时创建/关闭宠物窗口 */
async function onPetToggle(v: boolean) {
  try {
    await settings.setPetEnabled(v);
    if (v) message.success(t("settings.msgPetOn"));
    else message.info(t("settings.msgPetOff"));
  } catch (e) {
    message.error(String(e));
  }
}

/** 宠物大小：持久化并即时调整窗口（resize_pet 幂等，窗口未建时下次创建生效） */
async function onPetSizeChange(v: number) {
  try {
    await settings.setPetSize(v);
  } catch (e) {
    message.error(String(e));
  }
}

/** 宠物活跃度选项 */
const petActivityOptions = computed(() => [
  { label: t("settings.petActivityLow"), value: "low" },
  { label: t("settings.petActivityMedium"), value: "medium" },
  { label: t("settings.petActivityHigh"), value: "high" },
]);

/** 宠物活跃度：影响空闲动作频率 */
function onPetActivityChange(v: "low" | "medium" | "high") {
  settings.setPetConfig({ activity: v });
}

/** 昼夜节律开关：夜里更容易打盹并戴睡帽，早晨问早安 */
function onCircadianToggle(v: boolean) {
  settings.setPetConfig({ circadian: v });
}

/** 完成后自动打开输出目录开关 */
function onAutoOpenToggle(v: boolean) {
  settings.setOutdirConfig({ ...settings.outdir, autoOpen: v });
}

/** 同名文件策略选项 */
const conflictOptions = computed(() => [
  { label: t("settings.conflictOverwrite"), value: "overwrite" },
  { label: t("settings.conflictRename"), value: "rename" },
]);

/** 同名文件策略：覆盖 / 自动递增序号 */
function onConflictChange(v: "overwrite" | "rename") {
  settings.setOutdirConfig({ ...settings.outdir, conflict: v });
}

/** 任务完成通知开关 */
function onNotifyToggle(v: boolean) {
  settings.setNotifyOnComplete(v);
}

async function onAutostartChange(v: boolean) {
  autostartTouched = true;
  // 先同步更新开关再执行系统操作：受控开关若等 await 返回才更新，点击会显得"没反应"
  autostart.value = v;
  autostartBusy.value = true;
  try {
    if (v) {
      await enable();
      message.success(t("settings.msgAutostartOn"));
    } else {
      await disable();
      message.info(t("settings.msgAutostartOff"));
    }
    // 与系统实际状态对账：托盘菜单等外部改动后以 OS 为准，避免 UI 与注册表长期失配
    const actual = await isEnabled();
    if (actual !== v) autostart.value = actual;
  } catch (e) {
    // 系统操作失败：回滚开关并提示原因
    autostart.value = !v;
    message.error(String(e));
  } finally {
    autostartBusy.value = false;
  }
}

onMounted(async () => {
  getVersion().then((v) => { version.value = v; }).catch(() => { /* 非 Tauri 环境忽略 */ });
  try {
    const enabled = await isEnabled();
    // 只在用户尚未手动切换时采用系统实际状态，避免异步回写覆盖刚开启的开关
    if (!autostartTouched) autostart.value = enabled;
  } catch { /* 忽略 */ }
  if (settings.watcher.enabled && settings.watcher.folder) {
    await applyWatcher();
  }
  syncCloudConfig();
  syncLocalChatModel();
  localStatus.value = await localEngineStatus();
  await refreshEmbedStatus();
  await refreshChatStatus();
});
</script>

<style scoped>
/* ===== 左右分栏布局：左侧分类导航 + 右侧内容区 ===== */
.settings-panel {
  display: flex;
  align-items: flex-start;
  gap: 20px;
}
.settings-nav {
  flex: none;
  width: 150px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 10px;
  box-sizing: border-box;
}
.settings-nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  border: none;
  background: transparent;
  border-radius: 8px;
  padding: 9px 12px;
  font-size: 13px;
  color: var(--text-sub);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.settings-nav-item:hover {
  background: var(--bg-hover);
}
.settings-nav-item.active {
  background: var(--bg-active);
  color: var(--text-main);
  font-weight: 600;
}
.settings-content {
  flex: 1;
  min-width: 0;
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
.pet-size-row {
  width: 100%;
  max-width: 320px;
}
.pet-size-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--accent);
  white-space: nowrap;
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

/* ===== 个人资料页 ===== */
.profile-head {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 18px;
}
.profile-avatar {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: var(--accent);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  font-weight: 600;
  flex-shrink: 0;
}
.profile-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.profile-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.profile-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-main);
}
.profile-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--accent-soft);
  color: var(--accent);
}
.profile-sub {
  font-size: 12px;
  color: var(--text-muted);
}
.stat-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}
.stat-card {
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 12px;
  text-align: center;
}
.stat-num {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-main);
}
.stat-label {
  font-size: 11px;
  color: var(--text-muted);
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

/* ===== 云端服务商预设按钮 ===== */
.preset-row {
  flex-wrap: wrap;
}
.preset-btn {
  padding: 6px 14px;
  font-size: 12px;
  background: var(--bg-tag);
  color: var(--text-sub);
}
.preset-btn:hover {
  color: var(--accent);
}
.preset-btn.active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}
.preset-custom {
  font-size: 12px;
  color: var(--text-muted);
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
/* 纵向 flex 卡片中按钮默认被 stretch 拉满宽度，收回到内容宽度 */
.clear-history-btn {
  align-self: flex-start;
}
/* 全局快捷键：当前键位徽章与录制提示 */
.shortcut-badge {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 12px;
  border-radius: 12px;
  background: var(--accent-soft);
  color: var(--accent);
}
.shortcut-recording {
  font-size: 13px;
  font-weight: 600;
  color: var(--orange);
}
:deep(.n-switch .n-switch__button) {
  border-radius: 50%;
}
</style>