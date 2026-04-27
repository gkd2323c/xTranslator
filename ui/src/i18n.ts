import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zhCN from "./locales/zh-CN/translation.json";
import en from "./locales/en/translation.json";

const STORAGE_KEY = "xtranslator-lang";

function getInitialLang(): string {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "zh-CN" || stored === "en") return stored;
  } catch { /* ok */ }
  return "zh-CN";
}

i18n.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: zhCN },
    en: { translation: en },
  },
  lng: getInitialLang(),
  fallbackLng: "zh-CN",
  interpolation: { escapeValue: false },
});

export function setI18nLanguage(lang: string) {
  try { localStorage.setItem(STORAGE_KEY, lang); } catch { /* ok */ }
  i18n.changeLanguage(lang);
}

export default i18n;
