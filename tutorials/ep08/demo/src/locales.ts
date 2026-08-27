/**
 * EP08：双语文案。成品是四语（中/英/日/韩），结构完全一样。
 *
 * 约定：主窗口用 app.* 段，宠物用 pet.* 段，
 * 两个窗口共享同一个 i18n 实例的定义。
 */
export type LocaleKey = "zh-CN" | "en-US";

export const messages: Record<LocaleKey, Record<string, any>> = {
  "zh-CN": {
    app: {
      title: "文档工具箱",
      subtitle: "EP08 · 多语言与主题",
      runSuccess: "模拟任务（成功）",
      runFail: "模拟任务（失败）",
      running: "任务进行中…",
      done: "任务完成 ✅",
      failed: "任务失败 ❌",
      theme: "切换主题",
      lang: "English",
    },
    pet: {
      start: "开工啦！",
      working: "处理中 {pct}%",
      done: "{name}搞定！✨",
      error: "呜…出错了，抱抱你，再试一次？",
    },
  },
  "en-US": {
    app: {
      title: "Doc Toolbox",
      subtitle: "EP08 · i18n & Theme",
      runSuccess: "Run task (success)",
      runFail: "Run task (failure)",
      running: "Working…",
      done: "Done ✅",
      failed: "Failed ❌",
      theme: "Toggle theme",
      lang: "中文",
    },
    pet: {
      start: "Let's go!",
      working: "Working {pct}%",
      done: "{name} done! ✨",
      error: "Oops… something broke. Hug, then retry?",
    },
  },
};
