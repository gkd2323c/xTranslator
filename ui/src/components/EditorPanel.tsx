import { useState, useEffect, useCallback, useMemo } from "react";
import { useAppStore, computeTranslationProgress } from "../stores/appStore";
import { updateTranslation, heuristicSearch, translateString, setApiKey, tcscConvert, checkAliases, type HeuristicMatchDTO, type AliasCheckResult } from "../api/strings";
import { Save, X, Type, Search, Copy, Languages, Key, AlertTriangle, Loader } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { ProgressBar } from "./ProgressBar";

export function EditorPanel() {
  const { t } = useTranslation();
  const selectedItem = useAppStore((s) => s.selectedItem);
  const selectedId = useAppStore((s) => s.selectedId);
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  const updateItemTranslation = useAppStore((s) => s.updateItemTranslation);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const allItems = useAppStore((s) => s.allItems);

  const translationProgress = useMemo(() => computeTranslationProgress(allItems), [allItems]);

  const [localTrans, setLocalTrans] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [isTranslating, setIsTranslating] = useState(false);
  const [aliasResult, setAliasResult] = useState<AliasCheckResult | null>(null);
  const [matches, setMatches] = useState<HeuristicMatchDTO[]>([]);
  const [showApiKeyDialog, setShowApiKeyDialog] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");

  useEffect(() => {
    setLocalTrans(selectedItem?.translation || "");
    setMatches([]);
    setAliasResult(null);
    // Check alias integrity when selecting a new string
    if (selectedItem) {
      checkAliases(selectedItem.id).then(setAliasResult).catch(() => {});
    }
  }, [selectedId]);

  const handleSave = useCallback(async () => {
    if (selectedId === null || !selectedItem) return;
    setIsSaving(true);
    try {
      await updateTranslation(selectedItem.id, localTrans);
      updateItemTranslation(selectedItem.id, localTrans);
      toast.success(t("editor.translationSaved"));
    } catch (e: any) {
      toast.error(`${t("editor.saveFailed")}: ${e}`);
    } finally {
      setIsSaving(false);
    }
  }, [selectedId, selectedItem, localTrans, updateItemTranslation]);

  const handleHeuristicSearch = useCallback(async () => {
    if (!selectedItem || selectedItem.status === "translated") return;
    setIsSearching(true);
    try {
      const results = await heuristicSearch({
        source: selectedItem.source,
        min_similarity: 0.4,
        max_results: 5,
      });
      setMatches(results);
      if (results.length === 0) toast(t("editor.noSimilarFound"));
    } catch (e: any) {
      toast.error(`${t("editor.searchFailed")}: ${e}`);
    } finally {
      setIsSearching(false);
    }
  }, [selectedItem]);

  const handleTranslate = useCallback(async () => {
    if (!selectedItem) return;
    setIsTranslating(true);
    try {
      const result = await translateString({
        text: selectedItem.source,
        source_lang: language,
        target_lang: targetLang,
      });
      setLocalTrans(result);
      toast.success(t("editor.machineTranslationDone"));
    } catch (e: any) {
      if (e.includes("API key")) {
        setShowApiKeyDialog(true);
      } else {
        toast.error(`${t("editor.translationFailed")}: ${e}`);
      }
    } finally {
      setIsTranslating(false);
    }
  }, [selectedItem]);

  const handleSetApiKey = useCallback(async () => {
    if (!apiKeyInput.trim()) {
      toast.error(t("editor.apiKeyEmpty"));
      return;
    }
    try {
      await setApiKey(apiKeyInput.trim());
      toast.success(t("editor.apiKeySaved"));
      setShowApiKeyDialog(false);
      setApiKeyInput("");
    } catch (e: any) {
      toast.error(`${t("editor.apiKeySetFailed")}: ${e}`);
    }
  }, [apiKeyInput]);

  const applyMatch = (translation: string) => {
    setLocalTrans(translation);
    toast.success(t("editor.translationCopied"));
  };

  // F2 快捷键
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "F2" && selectedItem) {
        const textarea = document.querySelector(".editor-textarea") as HTMLTextAreaElement;
        textarea?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedItem]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && e.key === "Enter") handleSave();
  };

  if (!selectedItem) {
    return (
      <div className="editor-panel editor-empty">
        <Type size={24} opacity={0.3} />
        <p>{t("editor.selectToEdit")}</p>
      </div>
    );
  }

  return (
    <div className="editor-panel">
      <div className="editor-header">
        <div className="editor-meta">
          <span className="editor-id">#{selectedItem.id}</span>
          <span className="editor-sig">
            {selectedItem.record_sig}:{selectedItem.field_sig}
          </span>
          <span className="editor-formid">{selectedItem.form_id}</span>
          <span className={`editor-status-badge badge-${selectedItem.status}`}>
            {selectedItem.status}
          </span>
        </div>
        <div className="editor-actions">
          <button onClick={() => setShowApiKeyDialog(true)} className="btn btn-ghost btn-sm" title={t("editor.setApiKeyTooltip")}>
            <Key size={14} />
          </button>
          {selectedItem.status !== "translated" && (
            <>
              <button onClick={handleTranslate} disabled={isTranslating} className="btn btn-sm" title={t("editor.machineTranslateTooltip")}>
                {isTranslating ? <Loader size={14} style={{ animation: "spin 1s linear infinite" }} /> : <Languages size={14} />}
                <span>{isTranslating ? "Translating..." : "Translate"}</span>
              </button>
              <button onClick={handleHeuristicSearch} disabled={isSearching} className="btn btn-sm" title={t("editor.findSimilarTooltip")}>
                <Search size={14} />
                <span>{isSearching ? "Searching..." : "Similar"}</span>
              </button>
            </>
          )}
          <button onClick={handleSave} disabled={isSaving} className="btn btn-primary btn-sm" title="Ctrl+Enter">
            <Save size={14} />
            <span>{t("editor.save")}</span>
          </button>
          <button onClick={() => setSelectedById(null)} className="btn btn-ghost btn-sm">
            <X size={14} />
          </button>
        </div>
      </div>

      <div className="editor-body">
        <div className="editor-source">
          <label>{t("common.source")}</label>
          <div className="editor-source-text">{selectedItem.source}</div>
        </div>
        <div className="editor-translation">
          <label>
            {t("common.translation")}
            {aliasResult && aliasResult.has_mismatch && (
              <span style={{ marginLeft: 8, color: "#e74c3c", fontSize: 12, fontWeight: "normal" }} title={aliasResult.missing_in_trans.join(", ")}>
                <AlertTriangle size={12} style={{ verticalAlign: "middle" }} /> {t("editor.aliasMismatch")}
              </span>
            )}
          </label>
          <textarea
            value={localTrans}
            onChange={(e) => setLocalTrans(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={3}
            className="editor-textarea"
            placeholder={t("editor.enterTranslation")}
            autoFocus
          />
          <div className="editor-tcsc-buttons">
            <button
              type="button"
              onClick={async () => {
                if (!localTrans) return;
                try {
                  const result = await tcscConvert(localTrans, "to_simplified");
                  setLocalTrans(result);
                  toast.success(t("editor.convertedToSimplified"));
                } catch (e: any) {
                  toast.error(`${t("editor.tcscFailed")}: ${e}`);
                }
              }}
              className="btn btn-ghost btn-xs"
              title={t("editor.tcsc_simplified")}
            >
              {t("editor.tcsc_simplified")}
            </button>
            <button
              type="button"
              onClick={async () => {
                if (!localTrans) return;
                try {
                  const result = await tcscConvert(localTrans, "to_traditional");
                  setLocalTrans(result);
                  toast.success(t("editor.convertedToTraditional"));
                } catch (e: any) {
                  toast.error(`${t("editor.tcscFailed")}: ${e}`);
                }
              }}
              className="btn btn-ghost btn-xs"
              title={t("editor.tcsc_traditional")}
            >
              {t("editor.tcsc_traditional")}
            </button>
          </div>
        </div>
        {matches.length > 0 && (
          <div className="editor-matches">
            <label>{t("editor.similarTranslations")}</label>
            <div className="matches-list">
              {matches.map((m, i) => (
                <div key={i} className="match-item" onClick={() => applyMatch(m.translation)}>
                  <div className="match-source" title={m.source}>{m.source}</div>
                  <div className="match-translation">{m.translation}</div>
                  <div className="match-meta">
                    <span className="match-sim">{(m.similarity * 100).toFixed(0)}%</span>
                    <Copy size={12} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="editor-footer">
        <ProgressBar translated={translationProgress.translated} total={translationProgress.total} />
      </div>

      {showApiKeyDialog && (
        <div className="dialog-overlay" onClick={() => setShowApiKeyDialog(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <h3>{t("editor.setTranslationApiKey")}</h3>
            <p className="dialog-hint">{t("editor.apiKeyHint")}</p>
            <input
              type="password"
              value={apiKeyInput}
              onChange={(e) => setApiKeyInput(e.target.value)}
              placeholder={t("editor.skPlaceholder")}
              className="dialog-input"
              onKeyDown={(e) => { if (e.key === "Enter") handleSetApiKey(); }}
            />
            <div className="dialog-actions">
              <button onClick={() => setShowApiKeyDialog(false)} className="btn btn-ghost">{t("common.cancel")}</button>
              <button onClick={handleSetApiKey} className="btn btn-primary">{t("common.save")}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}