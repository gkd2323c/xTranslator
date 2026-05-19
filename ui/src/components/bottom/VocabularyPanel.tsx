import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Book, FolderOpen, Search, X } from "lucide-react";
import toast from "react-hot-toast";
import { loadVocabulary } from "../../api/strings";
import type { VocabularyInfo } from "../../api/strings";
import { useAppStore } from "../../stores/appStore";
import { Button, EmptyState } from "../ui";

export function VocabularyPanel() {
  const { t } = useTranslation();
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  const items = useAppStore((s) => s.items);
  const [info, setInfo] = useState<VocabularyInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState("");

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

  // 从当前 items 中抽取简短源文本作为词汇预览
  const vocabEntries = useMemo(() => {
    const seen = new Set<string>();
    const entries: { source: string; translation: string }[] = [];
    for (const item of items) {
      if (!item.source || item.source.length > 60) continue;
      const key = item.source.toLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);
      entries.push({
        source: item.source,
        translation: item.translation || "",
      });
      if (entries.length >= 200) break; // 限制预览数量
    }
    return entries;
  }, [items]);

  const filtered = useMemo(() => {
    if (!search) return vocabEntries;
    const q = search.toLowerCase();
    return vocabEntries.filter(
      (e) => e.source.toLowerCase().includes(q) || e.translation.toLowerCase().includes(q)
    );
  }, [vocabEntries, search]);

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
            <Button variant="ghost" size="sm" onClick={handleLoad} loading={loading} style={{ marginLeft: "auto" }}>
              {t("vocabularyPanel.reload")}
            </Button>
          </div>

          <div className="vocabulary-sources">
            {info.base_names.map((name, i) => (
              <div key={i} className="vocabulary-source-item">{name}</div>
            ))}
          </div>

          {/* 词汇搜索预览 */}
          <div className="vocabulary-search-section">
            <div className="vocabulary-search-bar">
              <Search size={14} className="vocabulary-search-icon" />
              <input
                type="text"
                className="vocabulary-search-input"
                placeholder={t("vocabularyPanel.searchPlaceholder", { defaultValue: "Search vocabulary..." })}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              {search && (
                <button className="vocabulary-search-clear" onClick={() => setSearch("")}>
                  <X size={14} />
                </button>
              )}
            </div>
            <div className="vocabulary-pairs-list">
              {filtered.length === 0 ? (
                <div className="vocabulary-pairs-empty">{t("vocabularyPanel.noResults", { defaultValue: "No matching entries" })}</div>
              ) : (
                filtered.map((entry, i) => (
                  <div key={i} className="vocabulary-pair-row">
                    <span className="vocabulary-pair-source">{entry.source}</span>
                    <span className="vocabulary-pair-arrow">→</span>
                    <span className={`vocabulary-pair-trans ${!entry.translation ? "vocabulary-pair-empty" : ""}`}>
                      {entry.translation || t("vocabularyPanel.untouched", { defaultValue: "—" })}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
