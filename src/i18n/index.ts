import { createI18n } from "vue-i18n";
import zhCN from "./locales/zh-CN";
import enUS from "./locales/en-US";
import jaJP from "./locales/ja-JP";
import koKR from "./locales/ko-KR";

export type LocaleCode = "zh-CN" | "en-US" | "ja-JP" | "ko-KR";
export const LOCALE_CODES: LocaleCode[] = ["zh-CN", "en-US", "ja-JP", "ko-KR"];

/** 根据系统语言匹配支持的语言，未匹配时默认简体中文 */
export function resolveSystemLocale(): LocaleCode {
  const lang = navigator.language;
  if (lang.startsWith("zh")) return "zh-CN";
  if (lang.startsWith("en")) return "en-US";
  if (lang.startsWith("ja")) return "ja-JP";
  if (lang.startsWith("ko")) return "ko-KR";
  return "zh-CN";
}

const i18n = createI18n({
  legacy: false,
  locale: resolveSystemLocale(),
  fallbackLocale: "zh-CN",
  messages: {
    "zh-CN": zhCN,
    "en-US": enUS,
    "ja-JP": jaJP,
    "ko-KR": koKR,
  },
});

export default i18n;
