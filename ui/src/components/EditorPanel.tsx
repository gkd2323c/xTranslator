import { useState, useEffect, useCallback, useMemo } from "react";
import { useAppStore, computeTranslationProgress } from "../stores/appStore";
import { updateTranslation, heuristicSearch, translateString, setApiKey, tcscConvert, rtlReverse, shapeArabic, deshapeArabic, checkAliases, type HeuristicMatchDTO, type AliasCheckResult } from "../api/strings";
import { Save, X, Type, Search, Languages, Key, AlertTriangle, ArrowRight, Copy, ArrowUp, ArrowDown } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { Button, Textarea, Badge, Modal, Input, ProgressBar, EmptyState } from "./ui";

const TAG_REGEX = /(<\/?[A-Za-z][^>]*>)/g;

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function highlightTags(text: string): string {
  return text
    .split(TAG_REGEX)
    .map((part) => {
      if (TAG_REGEX.test(part)) {
        TAG_REGEX.lastIndex = 0;
        return `<span class="tag-highlight">${escapeHtml(part)}</span>`;
      }
      return escapeHtml(part);
    })
    .join("");
}

export function EditorPanel() {
  const { t } = useTranslation();
  const selectedItem = useAppStore((s) => s.selectedItem);
  const selectedId = useAppStore((s) => s.selectedId);
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  const updateItemTranslation = useAppStore((s) => s.updateItemTranslation);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const allItems = useAppStore((s) => s.allItems);
  const items = useAppStore((s) => s.items);
  const dataConfigs = useAppStore((s) => s.dataConfigs);

  const translationProgress = useMemo(() => computeTranslationProgress(allItems), [allItems]);

  const [localTrans, setLocalTrans] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [isTranslating, setIsTranslating] = useState(false);
  const [aliasResult, setAliasResult] = useState<AliasCheckResult | null>(null);
  const [matches, setMatches] = useState<HeuristicMatchDTO[]>([]);
  const [showApiKeyDialog, setShowApiKeyDialog] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");

  const fieldSizeWarning = useMemo(() => {
    if (!selectedItem || !dataConfigs?.field_size_ref || !localTrans) return null;
    const key = `${selectedItem.record_sig}:${selectedItem.field_sig}`.toUpperCase();
    const info = dataConfigs.field_size_ref[key];
    if (!info) return null;
    const byteLen = new TextEncoder().encode(localTrans).length;
    if (byteLen > info.max_size) {
      return { max: info.max_size, current: byteLen };
    }
    return null;
  }, [selectedItem, dataConfigs, localTrans]);

  const jumpToUntranslated = useCallback((direction: "next" | "prev") => {
    if (!selectedId || items.length === 0) return;
    const currentIdx = items.findIndex((i) => i.id === selectedId);
    if (currentIdx === -1) return;
    const step = direction === "next" ? 1 : -1;
    for (let i = currentIdx + step; i >= 0 && i < items.length; i += step) {
      if (!items[i].translation) {
        setSelectedById(items[i].id);
        return;
      }
    }
    toast(t("editor.noMoreUntranslated"), { icon: "ℹ️" });
  }, [selectedId, items, setSelectedById]);

  useEffect(() => {
    setLocalTrans(selectedItem?.translation || "");
    setMatches([]);
    setAliasResult(null);
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

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "F2" && selectedItem) {
        const textarea = document.querySelector(".editor-textarea") as HTMLTextAreaElement;
        textarea?.focus();
      }
      if (e.ctrlKey && e.key === "ArrowDown") {
        e.preventDefault();
        jumpToUntranslated("next");
      }
      if (e.ctrlKey && e.key === "ArrowUp") {
        e.preventDefault();
        jumpToUntranslated("prev");
      }
      if (e.ctrlKey && e.key === "h" && selectedItem) {
        e.preventDefault();
        handleHeuristicSearch();
      }
      if (e.ctrlKey && e.key === "t" && selectedItem) {
        e.preventDefault();
        handleTranslate();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedItem, jumpToUntranslated, handleHeuristicSearch, handleTranslate]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && e.key === "Enter") handleSave();
  };

  if (!selectedItem) {
    return (
      <div className="editor-panel">
        <EmptyState
          icon={<Type size={24} />}
          title={t("editor.selectToEdit")}
        />
      </div>
    );
  }

  return (
    <div className="editor-panel">
      {/* Header: metadata only */}
      <div className="editor-header">
        <div className="editor-meta">
          <span className="editor-id">#{selectedItem.id}</span>
          <span className="editor-sig mono">
            {selectedItem.record_sig}:{selectedItem.field_sig}
          </span>
          <span className="editor-formid mono">{selectedItem.form_id}</span>
          <Badge variant={selectedItem.status === "translated" ? "translated" : selectedItem.status === "incomplete" ? "incomplete" : "locked"}>
            {selectedItem.status}
          </Badge>
          {aliasResult && aliasResult.has_mismatch && (
            <span className="editor-alias-warning" title={aliasResult.missing_in_trans.join(", ")}>
              <AlertTriangle size={12} /> {t("editor.aliasMismatch")}
            </span>
          )}
        </div>
        <Button variant="ghost" size="sm" onClick={() => setSelectedById(null)} icon={<X size={14} />} />
      </div>

      {/* Body: source | vertical toolbar | translation */}
      <div className="editor-body">
        <div className="editor-source">
          <label>{t("common.source")}</label>
          <div className="editor-source-text" dangerouslySetInnerHTML={{ __html: highlightTags(selectedItem.source) }} />
        </div>

        <div className="editor-vtoolbar">
          <button
            className="vtoolbar-btn"
            onClick={handleHeuristicSearch}
            disabled={isSearching || selectedItem.status === "translated"}
            title={t("editor.findSimilarTooltip")}
          >
            <Search size={14} />
          </button>
          <button
            className="vtoolbar-btn"
            onClick={handleTranslate}
            disabled={isTranslating}
            title={t("editor.machineTranslateTooltip")}
          >
            <Languages size={14} />
          </button>
          <button
            className="vtoolbar-btn"
            onClick={() => setLocalTrans(selectedItem.source)}
            title={t("editor.copySourceTooltip")}
          >
            <ArrowRight size={14} />
          </button>
          <div className="vtoolbar-sep" />
          <button
            className="vtoolbar-btn"
            onClick={() => jumpToUntranslated("prev")}
            title="Ctrl+↑"
          >
            <ArrowUp size={14} />
          </button>
          <button
            className="vtoolbar-btn"
            onClick={() => jumpToUntranslated("next")}
            title="Ctrl+↓"
          >
            <ArrowDown size={14} />
          </button>
          <div className="vtoolbar-sep" />
          <button
            className="vtoolbar-btn"
            onClick={() => setShowApiKeyDialog(true)}
            title={t("editor.setApiKeyTooltip")}
          >
            <Key size={14} />
          </button>
        </div>

        <div className="editor-translation">
          <label>{t("common.translation")}</label>
          <Textarea
            value={localTrans}
            onChange={(e) => setLocalTrans(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={3}
            className="editor-textarea"
            placeholder={t("editor.enterTranslation")}
            autoFocus
          />
          <div className="editor-info-row">
            <span className="editor-char-count">
              {t("editor.sourceChars")}: {selectedItem.source.length} | {t("editor.transChars")}: {localTrans.length}
            </span>
            {fieldSizeWarning && (
              <span className="editor-field-warning" title={t("editor.fieldSizeExceeded")}>
                <AlertTriangle size={12} /> {t("editor.fieldSizeWarning", { current: fieldSizeWarning.current, max: fieldSizeWarning.max })}
              </span>
            )}
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

      {/* Footer: TCSC/RTL + actions + progress */}
      <div className="editor-footer">
        <div className="editor-footer-row">
          <div className="editor-tcsc-buttons">
            <Button
              variant="ghost"
              size="xs"
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
              title={t("editor.tcsc_simplified")}
            >
              {t("editor.tcsc_simplified")}
            </Button>
            <Button
              variant="ghost"
              size="xs"
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
              title={t("editor.tcsc_traditional")}
            >
              {t("editor.tcsc_traditional")}
            </Button>
            <Button
              variant="ghost"
              size="xs"
              onClick={async () => {
                if (!localTrans) return;
                try {
                  const result = await rtlReverse(localTrans);
                  setLocalTrans(result);
                  toast.success(t("editor.rtlApplied"));
                } catch (e: any) {
                  toast.error(`${t("editor.rtlFailed")}: ${e}`);
                }
              }}
              title={t("editor.rtlTooltip")}
            >
              RTL
            </Button>
            <Button
              variant="ghost"
              size="xs"
              onClick={async () => {
                if (!localTrans) return;
                try {
                  const result = await shapeArabic(localTrans);
                  setLocalTrans(result);
                  toast.success(t("editor.shapeApplied", { defaultValue: "Arabic shaped" }));
                } catch (e: any) {
                  toast.error(`${t("editor.shapeFailed", { defaultValue: "Shape failed" })}: ${e}`);
                }
              }}
              title={t("editor.shapeTooltip", { defaultValue: "Shape Arabic (logical → presentation forms)" })}
            >
              Shape
            </Button>
            <Button
              variant="ghost"
              size="xs"
              onClick={async () => {
                if (!localTrans) return;
                try {
                  const result = await deshapeArabic(localTrans);
                  setLocalTrans(result);
                  toast.success(t("editor.deshapeApplied", { defaultValue: "Arabic deshaped" }));
                } catch (e: any) {
                  toast.error(`${t("editor.deshapeFailed", { defaultValue: "Deshape failed" })}: ${e}`);
                }
              }}
              title={t("editor.deshapeTooltip", { defaultValue: "Deshape Arabic (presentation forms → logical)" })}
            >
              Deshape
            </Button>
          </div>
          <div className="editor-actions">
            <Button
              variant="primary"
              size="sm"
              onClick={handleSave}
              loading={isSaving}
              title="Ctrl+Enter"
              icon={isSaving ? undefined : <Save size={14} />}
            >
              {t("editor.save")}
            </Button>
          </div>
        </div>
        <ProgressBar
          value={translationProgress.translated}
          max={translationProgress.total}
          variant="gradient"
          size="sm"
          showLabel
          label={t("sidebar.progress")}
        />
      </div>

      <Modal
        open={showApiKeyDialog}
        onClose={() => setShowApiKeyDialog(false)}
        title={t("editor.setTranslationApiKey")}
        size="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setShowApiKeyDialog(false)}>{t("common.cancel")}</Button>
            <Button variant="primary" onClick={handleSetApiKey}>{t("common.save")}</Button>
          </>
        }
      >
        <p className="ui-modal-hint">{t("editor.apiKeyHint")}</p>
        <Input
          type="password"
          value={apiKeyInput}
          onChange={(e) => setApiKeyInput(e.target.value)}
          placeholder={t("editor.skPlaceholder")}
          onKeyDown={(e) => { if (e.key === "Enter") handleSetApiKey(); }}
        />
      </Modal>
    </div>
  );
}
