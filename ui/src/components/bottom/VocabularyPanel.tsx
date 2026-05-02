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
      toast.success(`Loaded ${result.pair_count} vocabulary pairs from ${result.base_names.length} source(s)`);
    } catch (e: any) {
      toast.error(`Failed to load vocabulary: ${e}`);
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
            title={t("bottomTabs.vocabulary", { defaultValue: "Vocabulary" })}
            hint="Load translation vocabulary from game string files"
          />
          <Button variant="default" size="sm" onClick={handleLoad} loading={loading} icon={<FolderOpen size={14} />}>
            Load Vocabulary
          </Button>
        </div>
      ) : (
        <div className="vocabulary-content">
          <div className="vocabulary-stats">
            <span className="vocabulary-stat">
              <strong>{info.pair_count.toLocaleString()}</strong> pairs
            </span>
            <span className="vocabulary-stat">
              <strong>{info.base_names.length}</strong> source(s)
            </span>
          </div>
          <div className="vocabulary-sources">
            {info.base_names.map((name, i) => (
              <div key={i} className="vocabulary-source-item">{name}</div>
            ))}
          </div>
          <Button variant="ghost" size="sm" onClick={handleLoad} loading={loading}>
            Reload
          </Button>
        </div>
      )}
    </div>
  );
}
