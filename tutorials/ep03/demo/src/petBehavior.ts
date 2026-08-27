/**
 * EP03：宠物空闲行为引擎（纯函数，与 UI 解耦，方便单测）
 *
 * 设计原则：所有"随机"都以参数传入（rand: 0~1），
 * 组件里传 Math.random()，测试里传固定值 —— 行为完全可预测。
 */

/** 宠物空闲时的行为种类 */
export type PetBehavior = "idle" | "lookAround" | "hop" | "doze";

/**
 * 按权重抽取下一个行为：
 * lookAround 30%、hop 20%、doze 15%、其余保持 idle
 */
export function pickBehavior(rand: number): PetBehavior {
  if (rand < 0.3) return "lookAround";
  if (rand < 0.5) return "hop";
  if (rand < 0.65) return "doze";
  return "idle";
}

/** 本次行为结束后，多久（毫秒）再做下一个动作：4~10 秒 */
export function nextBehaviorDelay(rand: number): number {
  return 4000 + Math.floor(rand * 6000);
}

/** 每个行为的持续时长（毫秒），到期自动回到 idle */
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
