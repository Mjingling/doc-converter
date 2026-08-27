/**
 * 桌面宠物任务反馈事件桥（主窗口 → 宠物窗口）：
 * 通过 Tauri 全局事件 "pet-progress" 广播，宠物窗口据此显示进度条 / 完成庆祝 / 失败安慰。
 * 非 Tauri 环境（单测 / 浏览器预览）静默忽略。
 */
import { emit } from "@tauri-apps/api/event";

export type PetProgressPhase = "start" | "tick" | "done" | "error";

export interface PetProgressPayload {
  phase: PetProgressPhase;
  /** 0~100；缺省表示不确定态（单文件任务） */
  progress?: number;
  /** 任务展示名（完成/失败时显示在气泡里） */
  name?: string;
}

export async function emitPetProgress(payload: PetProgressPayload): Promise<void> {
  try {
    await emit("pet-progress", payload);
  } catch {
    /* 非 Tauri 环境忽略 */
  }
}
