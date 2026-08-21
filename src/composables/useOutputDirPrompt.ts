import { ref } from "vue";
import { useSettingsStore } from "../stores/settings";

/** 待处理的输出目录提示路径（非 null 时触发 Home.vue 中的对话框） */
export const pendingPromptPath = ref<string | null>(null);

/** 首次使用时触发输出目录提示（已设置过默认目录或已提示过则跳过） */
export function triggerOutputDirPrompt(srcPath: string) {
  const settings = useSettingsStore();
  if (settings.defaultOutDir || settings.outputDirPrompted) return;
  pendingPromptPath.value = srcPath;
}

/** 确认使用指定目录作为默认输出目录 */
export function confirmOutputDir(dir: string) {
  const settings = useSettingsStore();
  settings.setDefaultOutDir(dir);
  settings.outputDirPrompted = true;
  settings.save();
  pendingPromptPath.value = null;
}

/** 跳过提示（标记为已提示，不设置默认目录） */
export function dismissOutputDirPrompt() {
  const settings = useSettingsStore();
  settings.outputDirPrompted = true;
  settings.save();
  pendingPromptPath.value = null;
}
