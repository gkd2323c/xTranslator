import { useMemo } from "react";
import { useAppStore, computeTranslationProgress } from "../stores/appStore";

export function StatusBar() {
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

  return (
    <div className="statusbar">
      <div className="statusbar-section statusbar-file" title={espPath || ""}>
        {fileName}
        {isDirty && <span className="statusbar-dirty" title="Unsaved changes"> ●</span>}
      </div>
      <div className="statusbar-section statusbar-progress">
        {progress.translated.toLocaleString()} / {progress.total.toLocaleString()} ({percentage}%)
      </div>
      <div className="statusbar-section statusbar-selection">
        {selectedId ? `#${selectedId}` : "—"}
      </div>
      <div className="statusbar-section statusbar-mode">
        {espMode ? "ESP" : "SST"}
      </div>
      <div className="statusbar-section statusbar-lang">
        {language} → {targetLang}
      </div>
      <div className="statusbar-section statusbar-theme">
        {theme}
      </div>
    </div>
  );
}
