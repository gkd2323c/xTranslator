import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zhCN from "./locales/zh-CN/translation.json";
import en from "./locales/en/translation.json";
import ja from "./locales/ja/translation.json";
import ko from "./locales/ko/translation.json";
import fr from "./locales/fr/translation.json";
import de from "./locales/de/translation.json";
import es from "./locales/es/translation.json";
import ru from "./locales/ru/translation.json";
import pl from "./locales/pl/translation.json";
import pt from "./locales/pt/translation.json";

const STORAGE_KEY = "xtranslator-lang";

/** 支持的语言列表。新增语言：1. 添加 import 和 resource 注册 2. 在 MenuBar LANGS 中添加条目 */
export const SUPPORTED_LANGS: Record<string, string> = {
  "zh-CN": "中文",
  en: "English",
  ja: "日本語",
  ko: "한국어",
  fr: "Français",
  de: "Deutsch",
  es: "Español",
  ru: "Русский",
  pl: "Polski",
  pt: "Português",
};

function getInitialLang(): string {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && stored in SUPPORTED_LANGS) return stored;
  } catch { /* ok */ }
  return "zh-CN";
}

i18n.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: zhCN },
    en: { translation: en },
    ja: { translation: ja },
    ko: { translation: ko },
    fr: { translation: fr },
    de: { translation: de },
    es: { translation: es },
    ru: { translation: ru },
    pl: { translation: pl },
    pt: { translation: pt },
  },
  lng: getInitialLang(),
  fallbackLng: "zh-CN",
  interpolation: { escapeValue: false },
});

import { saveConfig } from "./api/strings";

export function setI18nLanguage(lang: string) {
  try { localStorage.setItem(STORAGE_KEY, lang); } catch { /* ok */ }
  i18n.changeLanguage(lang);
  saveConfig({ language: lang }).catch(() => {});
}

export default i18n;
