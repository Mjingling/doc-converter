/**
 * EP06：跨窗口任务进度事件（与成品 src/utils/petProgress.ts 同构）
 *
 * 主窗口在任务节点发事件，宠物窗口监听并反应。
 * 事件是"广播事实"，不共享状态——谁关心谁监听。
 */
import { emit } from "@tauri-apps/api/event";

export type PetProgressPhase = "start" | "tick" | "done" | "error";

export interface PetProgressPayload {
  phase: PetProgressPhase;
  /** tick 阶段携带 0~100 的进度值 */
  progress?: number;
  /** done/error 阶段可携带任务名，用于生成台词 */
  name?: string;
}

export const PET_PROGRESS_EVENT = "pet-progress";

/** 发送进度事件；非 Tauri 环境（纯浏览器预览）静默忽略 */
export async function emitPetProgress(payload: PetProgressPayload): Promise<void> {
  try {
    await emit(PET_PROGRESS_EVENT, payload);
  } catch {
    /* ignore */
  }
}
