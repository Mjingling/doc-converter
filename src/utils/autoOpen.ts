import { openPath } from "../api";

/**
 * 任务完成后按设置自动打开输出目录（共享工具）：
 * - 设置里关闭「完成后自动打开输出目录」时直接跳过
 * - 单文件：打开其所在目录；目录：直接打开
 * - 打开失败不影响主流程（静默）
 */
export async function maybeAutoOpenOutput(path: string): Promise<void> {
  try {
    const { useSettingsStore } = await import("../stores/settings");
    if (!useSettingsStore().outdir.autoOpen) return;
    const isDir = path.endsWith("/") || path.endsWith("\\");
    const target = isDir ? path : path.replace(/[/\\][^/\\]*$/, "");
    if (!target) return;
    await openPath(target);
  } catch {
    /* 自动打开失败不影响主流程 */
  }
}
