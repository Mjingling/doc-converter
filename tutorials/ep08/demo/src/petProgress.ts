/**
 * EP08：跨窗口任务进度事件（与 EP06 相同，原样保留）
 */
import { emit } from "@tauri-apps/api/event";

export type PetProgressPhase = "start" | "tick" | "done" | "error";

export interface PetProgressPayload {
  phase: PetProgressPhase;
  progress?: number;
  name?: string;
}

export const PET_PROGRESS_EVENT = "pet-progress";

export async function emitPetProgress(payload: PetProgressPayload): Promise<void> {
  try {
    await emit(PET_PROGRESS_EVENT, payload);
  } catch {
    /* ignore */
  }
}
