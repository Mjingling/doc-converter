/** 当前运行平台判断（Tauri WebView 内 navigator.userAgent 与系统一致） */
export const isMac =
  typeof navigator !== "undefined" && /Macintosh|Mac OS/i.test(navigator.userAgent);
