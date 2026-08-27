import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * 长任务完成的系统通知（共享工具）：
 * - 窗口可见时跳过（页面内已有成功提示，无需打扰）
 * - 无通知权限时静默跳过；通知失败不影响主流程
 */
export async function notifyDone(title: string, body: string): Promise<void> {
  try {
    if (await getCurrentWindow().isVisible()) return;
    let granted = await isPermissionGranted();
    if (!granted) granted = (await requestPermission()) === "granted";
    if (!granted) return;
    sendNotification({ title, body });
  } catch {
    /* 通知失败不影响主流程 */
  }
}
