import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAppStore, computeTranslationProgress } from "../stores/appStore";
import { updateTranslation, heuristicSearch, translateString, setApiKey, tcscConvert, rtlReverse, shapeArabic, deshapeArabic, checkAliases, spellCheckText, spellCheckSuggestions, spellCheckIgnore, type HeuristicMatchDTO, type AliasCheckResult, type SpellCheckResultDto, type SpellFaultDto } from "../api/strings";
import { Save, Search, Languages, Key, AlertTriangle, ArrowRight, Copy, ArrowUp, ArrowDown, Sparkles } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { Button, Textarea, Badge, Modal, Input, ProgressBar } from "./ui";

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

export interface EditorDialogProps {
  open: boolean;
  onClose: () => void;
}

export function EditorDialog({ open, onClose }: EditorDialogProps) {
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

  // ── Spell Check State ──
  const [spellResult, setSpellResult] = useState<SpellCheckResultDto | null>(null);
  const [selectedFaultIdx, setSelectedFaultIdx] = useState<number | null>(null);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [suggestionsLoading, setSuggestionsLoading] = useState(false);
  const spellTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const ignorePathRef = useRef<string>("");

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
    setSpellResult(null);
    setSelectedFaultIdx(null);
    setSuggestions([]);
    if (selectedItem) {
      checkAliases(selectedItem.id).then(setAliasResult).catch(() => {});
    }
  }, [selectedId]);

  // ── Spell Check: debounced check on text change ──
  const doSpellCheck = useCallback(async (text: string) => {
    if (!text) {
      setSpellResult(null);
      return;
    }
    try {
      const result = await spellCheckText(text);
      if (result.active && result.faults.length > 0) {
        setSpellResult(result);
      } else if (result.active && result.faults.length === 0) {
        // Show clean status
        setSpellResult(result);
      } else {
        setSpellResult(null);
      }
    } catch {
      // silently fail
    }
  }, []);

  useEffect(() => {
    if (spellTimerRef.current) {
      clearTimeout(spellTimerRef.current);
    }
    spellTimerRef.current = setTimeout(() => {
      doSpellCheck(localTrans);
    }, 500);
    return () => {
      if (spellTimerRef.current) clearTimeout(spellTimerRef.current);
    };
  }, [localTrans, doSpellCheck]);

  // ── Spell Check: fetch suggestions for selected fault ──
  const handleSelectFault = useCallback(async (index: number) => {
    if (!spellResult) return;
    setSelectedFaultIdx(index);
    setSuggestionsLoading(true);
    try {
      const suggs = await spellCheckSuggestions(spellResult.faults[index].word);
      setSuggestions(suggs);
    } catch {
      setSuggestions([]);
    } finally {
      setSuggestionsLoading(false);
    }
  }, [spellResult]);

  // ── Spell Check: apply suggestion ──
  const handleApplySuggestion = useCallback((suggestion: string) => {
    if (selectedFaultIdx === null || !spellResult) return;
    const fault = spellResult.faults[selectedFaultIdx];
    const before = localTrans.slice(0, fault.start_byte);
    const after = localTrans.slice(fault.end_byte);
    const newText = before + suggestion + after;
    setLocalTrans(newText);
    setSelectedFaultIdx(null);
    setSuggestions([]);
  }, [selectedFaultIdx, spellResult, localTrans]);

  // ── Spell Check: ignore word ──
  const handleIgnoreWord = useCallback(async (word: string) => {
    try {
      await spellCheckIgnore(word, ignorePathRef.current || "SpellCheck/ignore.txt");
      // Remove from fault list
      setSpellResult((prev) => {
        if (!prev) return null;
        return {
          ...prev,
          faults: prev.faults.filter((f) => f.word !== word),
        };
      });
      setSelectedFaultIdx(null);
      setSuggestions([]);
      toast.success(t("spellcheck.wordIgnored", { defaultValue: "Word added to ignore list" }));
    } catch (e: any) {
      toast.error(`${t("spellcheck.ignoreFailed", { defaultValue: "Failed to ignore word" })}: ${e}`);
    }
  }, []);

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
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
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
  }, [open, selectedItem, jumpToUntranslated, handleHeuristicSearch, handleTranslate]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && e.key === "Enter") handleSave();
  };

  const title = selectedItem
    ? `#${selectedItem.id}  ${selectedItem.record_sig}:${selectedItem.field_sig}`
    : t("editor.selectToEdit");

  return (
    <Modal open={open} onClose={onClose} title={title} size="xl">
      {!selectedItem ? (
        <div className="editor-empty-dialog">{t("editor.selectToEdit")}</div>
      ) : (
        <div className="editor-dialog-body">
          <div className="editor-dialog-header">
            <span className="editor-formid mono">{selectedItem.form_id}</span>
            <Badge variant={selectedItem.status === "translated" ? "translated" : selectedItem.status === "incomplete" ? "incomplete" : "locked"}>
              {selectedItem.status}
            </Badge>
            {selectedItem.is_vmad && <Badge variant="script" size="sm">VMAD</Badge>}
            {aliasResult && aliasResult.has_mismatch && (
              <span className="editor-alias-warning" title={aliasResult.missing_in_trans.join(", ")}>
                <AlertTriangle size={12} /> {t("editor.aliasMismatch")}
              </span>
            )}
          </div>

          <div className="editor-dialog-main">
            <div className="editor-dialog-left">
              <div className="editor-source">
                <label>{t("common.source")}</label>
                <div className="editor-source-text" dangerouslySetInnerHTML={{ __html: highlightTags(selectedItem.source) }} />
              </div>

              <div className="editor-translation">
                <label>{t("common.translation")}</label>
                <Textarea
                  value={localTrans}
                  onChange={(e) => setLocalTrans(e.target.value)}
                  onKeyDown={handleKeyDown}
                  rows={4}
                  className="editor-textarea"
                  placeholder={t("editor.enterTranslation")}
                  autoFocus
                />
              </div>
            </div>

            <div className="editor-dialog-right">
              <div className="editor-dialog-actions">
                <button
                  className="editor-action-btn"
                  onClick={handleHeuristicSearch}
                  disabled={isSearching || selectedItem.status === "translated"}
                  title={t("editor.findSimilarTooltip")}
                >
                  <Search size={16} />
                  <span>{t("editor.findSimilar", { defaultValue: "Find Similar" })}</span>
                </button>
                <button
                  className="editor-action-btn"
                  onClick={handleTranslate}
                  disabled={isTranslating}
                  title={t("editor.machineTranslateTooltip")}
                >
                  <Languages size={16} />
                  <span>{t("editor.machineTranslate", { defaultValue: "Translate" })}</span>
                </button>
                <button
                  className="editor-action-btn"
                  onClick={() => setLocalTrans(selectedItem.source)}
                  title={t("editor.copySourceTooltip")}
                >
                  <ArrowRight size={16} />
                  <span>{t("editor.copySource", { defaultValue: "Copy Source" })}</span>
                </button>
                <div className="editor-action-sep" />
                <button
                  className="editor-action-btn"
                  onClick={() => jumpToUntranslated("prev")}
                  title="Ctrl+↑"
                >
                  <ArrowUp size={16} />
                  <span>{t("editor.prevUntranslated", { defaultValue: "Prev" })}</span>
                </button>
                <button
                  className="editor-action-btn"
                  onClick={() => jumpToUntranslated("next")}
                  title="Ctrl+↓"
                >
                  <ArrowDown size={16} />
                  <span>{t("editor.nextUntranslated", { defaultValue: "Next" })}</span>
                </button>
                <div className="editor-action-sep" />
                <button
                  className="editor-action-btn"
                  onClick={() => setShowApiKeyDialog(true)}
                  title={t("editor.setApiKeyTooltip")}
                >
                  <Key size={16} />
                  <span>{t("editor.setApiKey", { defaultValue: "API Key" })}</span>
                </button>
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
          </div>

          {/* ── Spell Check Section ── */}
          {spellResult && spellResult.active && (
            <div className="editor-spellcheck">
              <div className="spellcheck-summary">
                <Sparkles size={14} />
                {spellResult.faults.length === 0 ? (
                  <span className="spellcheck-clean">{t("spellcheck.noErrors", { defaultValue: "No spelling errors" })}</span>
                ) : (
                  <span>
                    {t("spellcheck.errorsFound", { count: spellResult.faults.length, defaultValue: `{{count}} misspelled word(s)` }).replace("{{count}}", String(spellResult.faults.length))}
                  </span>
                )}
                {spellResult.fault_ratio_locked && (
                  <span className="spellcheck-locked" title={t("spellcheck.ratioLockedTooltip", { defaultValue: "Too many errors detected, highlighting paused" })}>
                    <AlertTriangle size={12} /> {t("spellcheck.ratioLocked", { defaultValue: "Ratio locked" })}
                  </span>
                )}
              </div>

              {spellResult.faults.length > 0 && (
                <div className="spellcheck-faults">
                  {spellResult.faults.map((fault: SpellFaultDto, idx: number) => (
                    <button
                      key={`${fault.word}-${idx}`}
                      className={`spellcheck-chip ${selectedFaultIdx === idx ? "spellcheck-chip-selected" : ""}`}
                      onClick={() => handleSelectFault(idx)}
                      title={t("spellcheck.clickForSuggestions", { defaultValue: "Click for suggestions" })}
                    >
                      {fault.word}
                    </button>
                  ))}
                </div>
              )}

              {/* Suggestions panel */}
              {selectedFaultIdx !== null && spellResult.faults[selectedFaultIdx] && (
                <div className="spellcheck-suggestions">
                  <div className="spellcheck-suggestions-header">
                    {t("spellcheck.suggestionsFor", { word: spellResult.faults[selectedFaultIdx].word, defaultValue: `Suggestions for "{{word}}"` }).replace("{{word}}", spellResult.faults[selectedFaultIdx].word)}
                  </div>
                  {suggestionsLoading ? (
                    <span className="spellcheck-loading">{t("common.searching")}</span>
                  ) : suggestions.length > 0 ? (
                    <div className="spellcheck-suggestions-list">
                      {suggestions.map((s, i) => (
                        <button
                          key={i}
                          className="spellcheck-suggestion-btn"
                          onClick={() => handleApplySuggestion(s)}
                        >
                          {s}
                        </button>
                      ))}
                    </div>
                  ) : (
                    <span className="spellcheck-no-suggestions">{t("spellcheck.noSuggestions", { defaultValue: "No suggestions" })}</span>
                  )}
                  <div className="spellcheck-suggestions-actions">
                    <Button
                      variant="ghost"
                      size="xs"
                      onClick={() => handleIgnoreWord(spellResult.faults[selectedFaultIdx].word)}
                    >
                      {t("spellcheck.ignore", { defaultValue: "Ignore" })}
                    </Button>
                  </div>
                </div>
              )}
            </div>
          )}

          <div className="editor-dialog-footer">
            <div className="editor-dialog-footer-left">
              <div className="editor-tcsc-buttons">
                <Button variant="ghost" size="xs" onClick={async () => {
                  if (!localTrans) return;
                  try {
                    const result = await tcscConvert(localTrans, "to_simplified");
                    setLocalTrans(result);
                    toast.success(t("editor.convertedToSimplified"));
                  } catch (e: any) { toast.error(`${t("editor.tcscFailed")}: ${e}`); }
                }} title={t("editor.tcsc_simplified")}>
                  {t("editor.tcsc_simplified")}
                </Button>
                <Button variant="ghost" size="xs" onClick={async () => {
                  if (!localTrans) return;
                  try {
                    const result = await tcscConvert(localTrans, "to_traditional");
                    setLocalTrans(result);
                    toast.success(t("editor.convertedToTraditional"));
                  } catch (e: any) { toast.error(`${t("editor.tcscFailed")}: ${e}`); }
                }} title={t("editor.tcsc_traditional")}>
                  {t("editor.tcsc_traditional")}
                </Button>
                <Button variant="ghost" size="xs" onClick={async () => {
                  if (!localTrans) return;
                  try {
                    const result = await rtlReverse(localTrans);
                    setLocalTrans(result);
                    toast.success(t("editor.rtlApplied"));
                  } catch (e: any) { toast.error(`${t("editor.rtlFailed")}: ${e}`); }
                }} title={t("editor.rtlTooltip")}>RTL</Button>
                <Button variant="ghost" size="xs" onClick={async () => {
                  if (!localTrans) return;
                  try {
                    const result = await shapeArabic(localTrans);
                    setLocalTrans(result);
                    toast.success(t("editor.shapeApplied", { defaultValue: "Arabic shaped" }));
                  } catch (e: any) { toast.error(`${t("editor.shapeFailed", { defaultValue: "Shape failed" })}: ${e}`); }
                }} title={t("editor.shapeTooltip", { defaultValue: "Shape Arabic" })}>Shape</Button>
                <Button variant="ghost" size="xs" onClick={async () => {
                  if (!localTrans) return;
                  try {
                    const result = await deshapeArabic(localTrans);
                    setLocalTrans(result);
                    toast.success(t("editor.deshapeApplied", { defaultValue: "Arabic deshaped" }));
                  } catch (e: any) { toast.error(`${t("editor.deshapeFailed", { defaultValue: "Deshape failed" })}: ${e}`); }
                }} title={t("editor.deshapeTooltip", { defaultValue: "Deshape Arabic" })}>Deshape</Button>
              </div>
              <span className="editor-char-count">
                {t("editor.sourceChars")}: {selectedItem.source.length} | {t("editor.transChars")}: {localTrans.length}
                {fieldSizeWarning && (
                  <span className="editor-field-warning" title={t("editor.fieldSizeExceeded")}>
                    <AlertTriangle size={12} /> {fieldSizeWarning.current}/{fieldSizeWarning.max}
                  </span>
                )}
              </span>
              {fieldSizeWarning && (
                <div className="editor-field-bar">
                  <div
                    className={`editor-field-bar-fill ${fieldSizeWarning.current > fieldSizeWarning.max ? "editor-field-bar-over" : ""}`}
                    style={{ width: `${Math.min(100, (fieldSizeWarning.current / fieldSizeWarning.max) * 100)}%` }}
                  />
                </div>
              )}
            </div>
            <div className="editor-dialog-footer-right">
              <ProgressBar
                value={translationProgress.translated}
                max={translationProgress.total}
                variant="gradient"
                size="sm"
                showLabel
                label={t("sidebar.progress")}
              />
              <div className="editor-actions">
                <Button variant="ghost" size="sm" onClick={onClose}>{t("common.close", { defaultValue: "Close" })}</Button>
                <Button variant="primary" size="sm" onClick={handleSave} loading={isSaving} title="Ctrl+Enter" icon={isSaving ? undefined : <Save size={14} />}>
                  {t("editor.save")}
                </Button>
              </div>
            </div>
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
      )}
    </Modal>
  );
}
