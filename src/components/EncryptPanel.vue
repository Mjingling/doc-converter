<template>
  <div class="panel">
    <div class="panel-head">
      <h2>{{ t("encrypt.title") }}</h2>
      <p>{{ t("encrypt.subtitle") }}</p>
    </div>

    <!-- 工具切换：加密 / 解密 -->
    <div class="mode-tabs">
      <button
        v-for="m in modes"
        :key="m"
        class="mode-tab"
        :class="{ active: mode === m }"
        @click="mode = m"
      >
        <NIcon :component="m === 'encrypt' ? LockClosedOutline : LockOpenOutline" :size="14" />
        {{ t(`encrypt.mode${m === "encrypt" ? "Encrypt" : "Decrypt"}`) }}
      </button>
    </div>

    <!-- 上传区 -->
    <div class="upload-zone" @click="pickFile">
      <div v-if="!pdfFile" class="zone-empty">
        <NIcon :component="DocumentLockOutline" :size="34" color="#e6494c" />
        <p class="zone-main">
          {{ t("common.dropBefore") }}<span class="link">{{ t("common.dropMid") }}</span>{{ t("common.dropAfter") }}
        </p>
        <p class="zone-sub">{{ t("encrypt.dropSub") }}</p>
      </div>
      <div v-else class="zone-filled">
        <NIcon :component="DocumentTextOutline" :size="20" color="#e6494c" />
        <span class="fname" :title="pdfFile">{{ fileName }}</span>
        <span class="size-tag">PDF</span>
      </div>
    </div>

    <!-- 加密配置 -->
    <div v-if="pdfFile && mode === 'encrypt'" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("encrypt.userPassLabel") }}</label>
        <NInput
          v-model:value="userPass"
          type="password"
          show-password-on="click"
          :placeholder="t('encrypt.userPassPlaceholder')"
          size="large"
        />
      </div>
      <div class="config-row">
        <label class="config-label">{{ t("encrypt.ownerPassLabel") }}</label>
        <NInput
          v-model:value="ownerPass"
          type="password"
          show-password-on="click"
          :placeholder="t('encrypt.ownerPassPlaceholder')"
          size="large"
        />
        <span class="config-hint">{{ t("encrypt.ownerHint") }}</span>
      </div>
    </div>

    <!-- 解密配置 -->
    <div v-if="pdfFile && mode === 'decrypt'" class="config">
      <div class="config-row">
        <label class="config-label">{{ t("encrypt.passLabel") }}</label>
        <NInput
          v-model:value="userPass"
          type="password"
          show-password-on="click"
          :placeholder="t('encrypt.passPlaceholder')"
          size="large"
        />
      </div>
    </div>

    <!-- CTA -->
    <div class="action-row">
      <span class="hint">{{ t(mode === "encrypt" ? "encrypt.hintEncrypt" : "encrypt.hintDecrypt") }}</span>
      <button class="cta" :disabled="!pdfFile || running" @click="doWork">
        <NIcon :component="mode === 'encrypt' ? LockClosedOutline : LockOpenOutline" :size="17" />
        {{ running ? t(mode === "encrypt" ? "encrypt.runningEncrypt" : "encrypt.runningDecrypt") : t(mode === "encrypt" ? "encrypt.ctaEncrypt" : "encrypt.ctaDecrypt") }}
      </button>
    </div>

    <!-- 执行进度 -->
    <TaskProgress :running="running" indeterminate :label="t(mode === 'encrypt' ? 'encrypt.runningEncrypt' : 'encrypt.runningDecrypt')" />

    <!-- 结果栏：打开文件 / 打开目录 -->
    <ResultBar :text="resultText" :outputs="resultOutputs" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon, NInput, useMessage } from "naive-ui";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  DocumentLockOutline, DocumentTextOutline,
  LockClosedOutline, LockOpenOutline,
} from "@vicons/ionicons5";
import { pdfDecrypt, pdfEncrypt } from "../api";
import ResultBar from "./ResultBar.vue";
import TaskProgress from "./TaskProgress.vue";
import { useSettingsStore } from "../stores/settings";
import { useHistoryStore } from "../stores/history";
import { defaultOutputPath } from "../utils/file";
import { usePanelTask } from "../composables/usePanelTask";

const { t } = useI18n();
const message = useMessage();
/** 最近一次成功结果（驱动 ResultBar） */
const resultOutputs = ref<string[]>([]);
const resultText = ref("");
const settings = useSettingsStore();
const history = useHistoryStore();

type Mode = "encrypt" | "decrypt";
const modes: Mode[] = ["encrypt", "decrypt"];
const mode = ref<Mode>("encrypt");

const pdfFile = ref("");
const fileName = computed(() => pdfFile.value.split(/[\\/]/).pop() ?? pdfFile.value);
/** 打开密码（解密模式复用此输入） */
const userPass = ref("");
/** 所有者密码；留空时与打开密码相同 */
const ownerPass = ref("");

/** 执行状态：running + 进度条（加密/解密共用） */
const { running, run } = usePanelTask();
async function pickFile() {
  const p = await openDialog({ filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!p) return;
  pdfFile.value = String(p);
}

/** 拖拽入口（Home.vue 转发 tauri://drag-drop） */
function handleDrop(paths: string[]) {
  const pdf = paths.find((p) => /\.pdf$/i.test(p));
  if (!pdf) {
    message.warning(t("encrypt.warnOnlyPdf"));
    return;
  }
  pdfFile.value = pdf;
}
defineExpose({ handleDrop });

async function doWork() {
  if (!pdfFile.value) {
    message.warning(t("encrypt.warnNoFile"));
    return;
  }
  if (!userPass.value) {
    message.warning(t("encrypt.warnNoPass"));
    return;
  }
  const suffix = mode.value === "encrypt" ? "_encrypted" : "_decrypted";
  const outPath = defaultOutputPath(pdfFile.value, suffix, settings.defaultOutDir);
  await run(async () => {
    try {
      const out =
        mode.value === "encrypt"
          ? await pdfEncrypt(pdfFile.value, outPath, userPass.value, ownerPass.value || userPass.value)
          : await pdfDecrypt(pdfFile.value, outPath, userPass.value);
      const outName = out.split(/[\\/]/).pop() ?? out;
      resultText.value = t("encrypt.success", { name: outName });
      resultOutputs.value = [out];
      await history.add({ kind: mode.value, name: outName, inputs: [pdfFile.value], outputs: [out], ok: true });
    } catch (e) {
      message.error(t("encrypt.fail", { err: String(e) }));
      await history.add({ kind: mode.value, name: fileName.value, inputs: [pdfFile.value], outputs: [], ok: false });
    }
  });
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
.mode-tabs {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}
.mode-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--border-strong);
  background: var(--bg-panel);
  color: var(--text-sub);
  font-size: 13px;
  padding: 6px 18px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.mode-tab.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}
.upload-zone {
  margin-top: 16px;
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
