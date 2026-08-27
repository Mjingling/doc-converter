import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import AssistantAvatar from "./AssistantAvatar.vue";

/**
 * AssistantAvatar 组件测试：状态表情切换 / 尺寸差异 / 挂载卸载安全
 */
describe("AssistantAvatar", () => {
  it("idle：胶囊眼 + 光晕 + 身体 + 投影（大尺寸全套）", () => {
    const w = mount(AssistantAvatar, { props: { size: "lg", state: "idle" } });
    expect(w.find(".halo").exists()).toBe(true);
    expect(w.find(".shadow").exists()).toBe(true);
    expect(w.find(".body").exists()).toBe(true);
    // 胶囊眼（ellipse）
    expect(w.findAll(".eye").length).toBe(2);
    // 思考点 / 旋转环 / 弧线眼 / × 眼都不出现
    expect(w.find(".think-dots").exists()).toBe(false);
    expect(w.find(".ring-wrap").exists()).toBe(false);
    expect(w.find(".eye-stroke").exists()).toBe(false);
    expect(w.find(".x-eye").exists()).toBe(false);
  });

  it("thinking：眯眼（ry 缩小）+ 头顶思考点", () => {
    const w = mount(AssistantAvatar, { props: { size: "sm", state: "thinking" } });
    expect(w.find(".think-dots").exists()).toBe(true);
    expect(w.findAll(".think-dots circle").length).toBe(3);
    const eyes = w.findAll(".eye");
    expect(eyes.length).toBe(2);
    expect(eyes[0].attributes("ry")).toBe("6");
  });

  it("working：正常眼 + 旋转光环 + look-down", () => {
    const w = mount(AssistantAvatar, { props: { size: "sm", state: "working" } });
    expect(w.find(".ring-wrap").exists()).toBe(true);
    expect(w.findAll(".eye")[0].attributes("ry")).toBe("15");
    expect(w.find(".eyes.look-down").exists()).toBe(true);
  });

  it("success：^ ^ 弧线眼 + hop 弹跳", () => {
    const w = mount(AssistantAvatar, { props: { size: "sm", state: "success" } });
    expect(w.findAll(".eye-stroke").length).toBe(2);
    expect(w.find(".eye").exists()).toBe(false);
    expect(w.find(".bob.hop").exists()).toBe(true);
  });

  it("error：× × 眼 + 歪头 + droop", () => {
    const w = mount(AssistantAvatar, { props: { size: "sm", state: "error" } });
    expect(w.findAll(".x-eye").length).toBe(2);
    expect(w.find(".head.tilt").exists()).toBe(true);
    expect(w.find(".bob.droop").exists()).toBe(true);
  });

  it("小尺寸不渲染光晕 / 投影 / 身体", () => {
    const w = mount(AssistantAvatar, { props: { size: "sm", state: "idle" } });
    expect(w.find(".halo").exists()).toBe(false);
    expect(w.find(".shadow").exists()).toBe(false);
    expect(w.find(".body").exists()).toBe(false);
  });

  it("状态类名与状态色变量挂载", async () => {
    const w = mount(AssistantAvatar, { props: { size: "sm", state: "idle" } });
    expect(w.find(".avatar.state-idle").exists()).toBe(true);
    expect(w.find(".avatar").attributes("style")).toContain("var(--accent)");
    await w.setProps({ state: "error" });
    expect(w.find(".avatar.state-error").exists()).toBe(true);
    expect(w.find(".avatar").attributes("style")).toContain("var(--red)");
  });

  it("挂载/卸载安全（定时器与监听清理不报错）", () => {
    const w = mount(AssistantAvatar, { props: { size: "lg", state: "idle" } });
    w.unmount();
    // 卸载后再次渲染不同状态也正常
    const w2 = mount(AssistantAvatar, { props: { size: "sm", state: "thinking" } });
    expect(w2.find(".think-dots").exists()).toBe(true);
    w2.unmount();
  });
});
