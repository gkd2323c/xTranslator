import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore, computeTranslationProgress } from "../stores/appStore";

const LANGUAGE_CODES: Record<string, string> = {
  english: "en",
  chinese: "zh",
  japanese: "ja",
  korean: "ko",
  french: "fr",
  german: "de",
  spanish: "es",
  italian: "it",
  russian: "ru",
  polish: "pl",
  portuguese: "pt",
  brazilian: "pt-BR",
  czech: "cs",
  hungarian: "hu",
};

export function StatusBar() {
  const { t, i18n } = useTranslation();
  const espPath = useAppStore((s) => s.espPath);
  const allItems = useAppStore((s) => s.allItems);
  const espMode = useAppStore((s) => s.espMode);
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  const isDirty = useAppStore((s) => s.isDirty);
  const selectedId = useAppStore((s) => s.selectedId);
  const theme = useAppStore((s) => s.theme);

  const progress = useMemo(() => computeTranslationProgress(allItems), [allItems]);
  const percentage = progress.total > 0 ? ((progress.translated / progress.total) * 100).toFixed(1) : "0.0";
  const fileName = espPath ? espPath.split(/[\\/]/).pop() || "—" : "—";
  const normalizedTheme = theme === "dark" ? "obsidian" : theme;
  const languageLabel = useMemo(() => {
    let displayNames: Intl.DisplayNames | null = null;
    try {
      displayNames = new Intl.DisplayNames([i18n.resolvedLanguage || i18n.language || "en"], { type: "language" });
    } catch {
      displayNames = null;
    }

    const formatLanguage = (value: string) => {
      const code = LANGUAGE_CODES[value.toLowerCase()];
      return (code && displayNames?.of(code)) || value;
    };

    return `${formatLanguage(language)} → ${formatLanguage(targetLang)}`;
  }, [i18n.language, i18n.resolvedLanguage, language, targetLang]);

  return (
    <div className="statusbar">
      <div className="statusbar-section statusbar-file" title={espPath || ""}>
        {fileName}
        {isDirty && <span className="statusbar-dirty" title={t("app.unsavedChanges")}> ●</span>}
      </div>
      <div className="statusbar-section statusbar-progress">
        {progress.translated.toLocaleString()} / {progress.total.toLocaleString()} ({percentage}%)
      </div>
      <div className="statusbar-section statusbar-selection">
        {selectedId ? `#${selectedId}` : "—"}
      </div>
      <div className="statusbar-section statusbar-mode">
        {espMode ? t("sidebar.espMode") : t("sidebar.stringsMode")}
      </div>
      <div className="statusbar-section statusbar-lang">
        {languageLabel}
      </div>
      <div className="statusbar-section statusbar-theme">
        {t(`theme.${normalizedTheme}`)}
      </div>
    </div>
  );
}
