/**
 * EP04：行为引擎 + 戳一戳反应（在 EP03 基础上新增 pickPokeReaction）
 */

export type PetBehavior = "idle" | "lookAround" | "hop" | "doze";
export type PokeReaction = "hop" | "wiggle" | "hearts";

export function pickBehavior(rand: number): PetBehavior {
  if (rand < 0.3) return "lookAround";
  if (rand < 0.5) return "hop";
  if (rand < 0.65) return "doze";
  return "idle";
}

export function nextBehaviorDelay(rand: number): number {
  return 4000 + Math.floor(rand * 6000);
}

export function behaviorDuration(b: PetBehavior): number {
  switch (b) {
    case "lookAround":
      return 2200;
    case "hop":
      return 700;
    case "doze":
      return 9000;
    default:
      return 0;
  }
}

/**
 * 被戳时的反应：40% 跳一下、30% 扭一扭、30% 冒爱心
 */
export function pickPokeReaction(rand: number): PokeReaction {
  if (rand < 0.4) return "hop";
  if (rand < 0.7) return "wiggle";
  return "hearts";
}

/** 戳一戳的台词池 */
export const POKE_LINES = ["哎呀，干嘛呀~", "唔！", "嘿嘿，别戳啦", "我在认真工作呢！"] as const;

export function pickPokeLine(rand: number): string {
  return POKE_LINES[Math.floor(rand * POKE_LINES.length) % POKE_LINES.length];
}
