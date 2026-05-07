import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Book, FolderOpen } from "lucide-react";
import toast from "react-hot-toast";
import { loadVocabulary } from "../../api/strings";
import type { VocabularyInfo } from "../../api/strings";
import { useAppStore } from "../../stores/appStore";
import { Button, EmptyState } from "../ui";

export function VocabularyPanel() {
  const { t } = useTranslation();
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  const [info, setInfo] = useState<VocabularyInfo | null>(null);
  const [loading, setLoading] = useState(false);

  const handleLoad = async () => {
    setLoading(true);
    try {
      const result = await loadVocabulary("", language, targetLang);
      setInfo(result);
      toast.success(t("vocabularyPanel.loaded", { pairs: result.pair_count, sources: result.base_names.length }));
    } catch (e: any) {
      toast.error(t("vocabularyPanel.loadFailed", { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bottom-panel-inner">
      {!info ? (
        <div className="vocabulary-empty">
          <EmptyState
            icon={<Book size={32} />}
            title={t("bottomTabs.vocabulary")}
            hint={t("vocabularyPanel.emptyHint")}
          />
          <Button variant="default" size="sm" onClick={handleLoad} loading={loading} icon={<FolderOpen size={14} />}>
            {t("vocabularyPanel.load")}
          </Button>
        </div>
      ) : (
        <div className="vocabulary-content">
          <div className="vocabulary-stats">
            <span className="vocabulary-stat">
              <strong>{info.pair_count.toLocaleString()}</strong> {t("vocabularyPanel.pairs")}
            </span>
            <span className="vocabulary-stat">
              <strong>{info.base_names.length}</strong> {t("vocabularyPanel.sources")}
            </span>
          </div>
          <div className="vocabulary-sources">
            {info.base_names.map((name, i) => (
              <div key={i} className="vocabulary-source-item">{name}</div>
            ))}
          </div>
          <Button variant="ghost" size="sm" onClick={handleLoad} loading={loading}>
            {t("vocabularyPanel.reload")}
          </Button>
        </div>
      )}
    </div>
  );
}
