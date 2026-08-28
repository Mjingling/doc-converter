/**
 * 桌宠行为纯函数：空闲行为调度 + 显示状态优先级仲裁
 * （与窗口/定时器解耦，可独立单测；PetWindow 负责驱动）
 */
import type { AvatarState } from "../components/AssistantAvatar.vue";

/** 空闲行为种类 */
export type PetBehavior =
  | { kind: "lookAround"; duration: 2500 } // 左右张望
  | { kind: "doze"; duration: 6000 | 7000 | 8000 | 9000 } // 打盹（带 Zzz）
  | { kind: "hop"; duration: 900 } // 开心小跳
  | { kind: "wiggle"; duration: 1200 }; // 左右摇摆

/** 随机取一个空闲行为（rand ∈ [0,1) 便于测试注入） */
export function pickBehavior(rand: number): PetBehavior {
  const r = ((rand % 1) + 1) % 1; // 归一化到 [0,1)，容忍测试传入越界值
  if (r < 0.3) return { kind: "lookAround", duration: 2500 };
  if (r < 0.6) {
    const secs = [6000, 7000, 8000, 9000] as const;
    return { kind: "doze", duration: secs[Math.floor(r * 10) % 4] };
  }
  if (r < 0.8) return { kind: "hop", duration: 900 };
  return { kind: "wiggle", duration: 1200 };
}

/** 下一次空闲行为的等待时长（10~22s；rand ∈ [0,1)） */
export function nextBehaviorDelay(rand: number): number {
  const r = ((rand % 1) + 1) % 1;
  return 10_000 + Math.round(r * 12_000);
}

/** 戳一戳反应种类：小跳 / 摇摆 / 冒爱心 */
export type PokeReaction = "hop" | "wiggle" | "hearts";

/** 随机取一个戳一戳反应（rand ∈ [0,1) 便于测试注入） */
export function pickPokeReaction(rand: number): PokeReaction {
  const r = ((rand % 1) + 1) % 1;
  if (r < 0.4) return "hop";
  if (r < 0.7) return "wiggle";
  return "hearts";
}

/** 下一次随机小贴士的等待时长（40~70s；rand ∈ [0,1)） */
export function nextTipDelay(rand: number): number {
  const r = ((rand % 1) + 1) % 1;
  return 40_000 + Math.round(r * 30_000);
}

/* ---------- 二期：去邻居小行星串门 ---------- */

/** 邻居星球方位 */
export type VisitSide = "left" | "right";

/** 随机选一颗邻居星球（rand ∈ [0,1)） */
export function pickVisitSide(rand: number): VisitSide {
  const r = ((rand % 1) + 1) % 1;
  return r < 0.5 ? "left" : "right";
}

/** 下一次串门的等待时长（40~80s；空闲且在家才会真的去） */
export function nextVisitDelay(rand: number): number {
  const r = ((rand % 1) + 1) % 1;
  return 40_000 + Math.round(r * 40_000);
}

/** 串门逗留时长（8~14s，之后自己回家） */
export function visitDwellMs(rand: number): number {
  const r = ((rand % 1) + 1) % 1;
  return 8_000 + Math.round(r * 6_000);
}

/** AI 状态事件的临时展示时长：成功/出错短闪，思考/工作持续到状态切换 */
export function aiStateHoldMs(state: AvatarState): number | null {
  switch (state) {
    case "success":
      return 1200;
    case "error":
      return 1600;
    default:
      return null; // thinking / working / idle 持续显示直到下一条事件
  }
}

/**
 * 仲裁当前应显示的头像状态。优先级：AI 状态（未过期）> 空闲行为 > idle。
 * @param aiState   最近一次 AI 事件状态（null = 无）
 * @param aiUntil   AI 状态有效期时间戳（ms；null = 持续）
 * @param dozing    空闲行为当前是否为打盹
 * @param now       当前时间戳（ms，注入便于测试）
 */
export function resolveDisplayState(
  aiState: AvatarState | null,
  aiUntil: number | null,
  dozing: boolean,
  now: number,
): AvatarState {
  if (aiState && aiState !== "idle" && (aiUntil === null || now < aiUntil)) {
    return aiState;
  }
  return dozing ? "dozing" : "idle";
}
