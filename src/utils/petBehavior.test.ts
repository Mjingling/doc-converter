import { describe, it, expect } from "vitest";
import { pickBehavior, nextBehaviorDelay, aiStateHoldMs, resolveDisplayState, pickPokeReaction, nextTipDelay } from "./petBehavior";

describe("pickBehavior", () => {
  it("按概率区间返回四种行为", () => {
    expect(pickBehavior(0.1).kind).toBe("lookAround");
    expect(pickBehavior(0.5).kind).toBe("doze");
    expect(pickBehavior(0.7).kind).toBe("hop");
    expect(pickBehavior(0.9).kind).toBe("wiggle");
  });

  it("边界值与越界值都归一化处理不抛错", () => {
    expect(pickBehavior(0).kind).toBe("lookAround");
    expect(pickBehavior(0.999).kind).toBe("wiggle");
    expect(pickBehavior(-0.5).kind).toBe("doze"); // -0.5 → 0.5 doze 区间
    expect(["lookAround", "doze", "hop", "wiggle"]).toContain(pickBehavior(3.7).kind);
  });

  it("doze 时长限定在 6~9 秒档位", () => {
    for (const r of [0.31, 0.45, 0.55, 0.59]) {
      const b = pickBehavior(r);
      if (b.kind === "doze") expect([6000, 7000, 8000, 9000]).toContain(b.duration);
    }
  });
});

describe("nextBehaviorDelay", () => {
  it("等待时长在 10~22 秒之间", () => {
    expect(nextBehaviorDelay(0)).toBe(10000);
    expect(nextBehaviorDelay(0.999)).toBe(21988); // round(0.999×12000)=11988
    expect(nextBehaviorDelay(0.5)).toBe(16000);
  });
});

describe("pickPokeReaction", () => {
  it("按概率区间返回三种反应", () => {
    expect(pickPokeReaction(0.1)).toBe("hop");
    expect(pickPokeReaction(0.5)).toBe("wiggle");
    expect(pickPokeReaction(0.9)).toBe("hearts");
  });

  it("边界与越界值归一化不抛错", () => {
    expect(pickPokeReaction(0)).toBe("hop");
    expect(pickPokeReaction(0.999)).toBe("hearts");
    expect(["hop", "wiggle", "hearts"]).toContain(pickPokeReaction(-1.2));
  });
});

describe("nextTipDelay", () => {
  it("小贴士间隔在 40~70 秒之间", () => {
    expect(nextTipDelay(0)).toBe(40000);
    expect(nextTipDelay(0.999)).toBe(69970); // round(0.999×30000)=29970
    expect(nextTipDelay(0.5)).toBe(55000);
  });
});

describe("aiStateHoldMs", () => {
  it("成功短闪 1.2s、出错 1.6s、思考/工作持续", () => {
    expect(aiStateHoldMs("success")).toBe(1200);
    expect(aiStateHoldMs("error")).toBe(1600);
    expect(aiStateHoldMs("thinking")).toBeNull();
    expect(aiStateHoldMs("working")).toBeNull();
  });
});

describe("resolveDisplayState", () => {
  const NOW = 1_000_000;

  it("无 AI 事件：空闲行为优先（打盹）", () => {
    expect(resolveDisplayState(null, null, true, NOW)).toBe("dozing");
    expect(resolveDisplayState(null, null, false, NOW)).toBe("idle");
  });

  it("AI 持续状态（thinking/working）优先于打盹", () => {
    expect(resolveDisplayState("thinking", null, true, NOW)).toBe("thinking");
    expect(resolveDisplayState("working", null, true, NOW)).toBe("working");
  });

  it("AI 临时状态（success）在有效期内显示、过期后回退", () => {
    expect(resolveDisplayState("success", NOW + 500, true, NOW)).toBe("success");
    expect(resolveDisplayState("success", NOW - 1, true, NOW)).toBe("dozing");
    expect(resolveDisplayState("success", NOW - 1, false, NOW)).toBe("idle");
  });

  it("idle 事件视为无 AI 状态", () => {
    expect(resolveDisplayState("idle", null, true, NOW)).toBe("dozing");
    expect(resolveDisplayState("idle", null, false, NOW)).toBe("idle");
  });
});
