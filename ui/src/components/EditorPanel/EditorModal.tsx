/**
 * EditorModal — 弹窗模式编辑器
 *
 * 从 EditorPanel.tsx 迁移的渲染逻辑，使用 useEditorCore() hook 获取所有状态和处理器。
 * 提供完整的单字符串编辑界面：源文本高亮、翻译输入、相似搜索、机器翻译、拼写检查等。
 */

import { useEffect } from "react";
import { Save, Search, Languages, Key, AlertTriangle, ArrowRight, Copy, ArrowUp, ArrowDown, Sparkles } from "lucide-react";
import toast from "react-hot-toast";
import { Button, Textarea, Badge, Modal, Input, ProgressBar } from "../ui";
import { useEditorCore, highlightTags } from "./EditorCore";
import { tcscConvert, rtlReverse, shapeArabic, deshapeArabic, type SpellFaultDto } from "../../api/strings";

// ============================================================================
// EditorModal 组件 Props 接口
// ============================================================================

export interface EditorModalProps {
  open: boolean;
  onClose: () => void;
}

// ============================================================================
// EditorModal 组件
// ============================================================================

export function EditorModal({ open, onClose }: EditorModalProps) {
  // 从共享 hook 获取所有状态和处理器
  const {
    selectedItem,
    translationProgress,
    localTrans,
    setLocalTrans,
    isSaving,
    handleSave,
    isSearching,
    matches,
    handleHeuristicSearch,
    isTranslating,
    handleTranslate,
    showApiKeyDialog,
    setShowApiKeyDialog,
    apiKeyInput,
    setApiKeyInput,
    handleSetApiKey,
    aliasResult,
    spellResult,
    selectedFaultIdx,
    suggestions,
    suggestionsLoading,
    handleSelectFault,
    handleApplySuggestion,
    handleIgnoreWord,
    fieldSizeWarning,
    jumpToUntranslated,
    applyMatch,
    t,
  } = useEditorCore();

  // ========== 键盘快捷键 ==========

  /**
   * 全局键盘快捷键（弹窗打开时激活）
   *   - Ctrl+↓：跳转到下一个未翻译项
   *   - Ctrl+↑：跳转到上一个未翻译项
   *   - Ctrl+H：启动相似翻译搜索
   *   - Ctrl+T：启动机器翻译
   */
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

  /**
   * 编辑框内本地键盘事件
   *   - Ctrl+Enter：保存翻译
   *   - Tab：插入两个空格
   */
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.ctrlKey && e.key === "Enter") {
      handleSave();
      return;
    }
    if (e.key === "Tab") {
      e.preventDefault();
      const textarea = e.currentTarget;
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const newValue = localTrans.substring(0, start) + "  " + localTrans.substring(end);
      setLocalTrans(newValue);
      // 状态更新后恢复光标位置
      requestAnimationFrame(() => {
        textarea.selectionStart = textarea.selectionEnd = start + 2;
      });
    }
  };

  // ========== 渲染 ==========

  // 对话框标题：显示字符串 ID 和签名
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
            <div className="editor-meta-row">
              <span className="editor-meta-tag">
                <span className="editor-meta-label">FormID:</span>
                <span className="editor-meta-value mono">{selectedItem.form_id}</span>
              </span>
              <span className="editor-meta-tag">
                <span className="editor-meta-label">Rec:</span>
                <span className="editor-meta-value">{selectedItem.record_sig}</span>
              </span>
              <span className="editor-meta-tag">
                <span className="editor-meta-label">Field:</span>
                <span className="editor-meta-value">{selectedItem.field_sig}</span>
              </span>
              <span className="editor-meta-tag">
                <span className="editor-meta-label">Type:</span>
                <span className="editor-meta-value">{["STRINGS", "DLSTRINGS", "ILSTRINGS"][selectedItem.list_index] || "STRINGS"}</span>
              </span>
              {fieldSizeWarning && (
                <span className="editor-meta-tag editor-meta-size">
                  <span className="editor-meta-label">Size:</span>
                  <span className="editor-size-bar-bg">
                    <span
                      className={`editor-size-bar-fill ${fieldSizeWarning.current > fieldSizeWarning.max ? "editor-size-bar-over" : ""}`}
                      style={{ width: `${Math.min(100, (fieldSizeWarning.current / fieldSizeWarning.max) * 100)}%` }}
                    />
                  </span>
                  <span className="editor-meta-value mono">{fieldSizeWarning.current}/{fieldSizeWarning.max}</span>
                </span>
              )}
            </div>
            <div className="editor-meta-status">
              <Badge variant={selectedItem.status === "translated" ? "translated" : selectedItem.status === "incomplete" ? "incomplete" : "locked"}>
                {selectedItem.status}
              </Badge>
              {selectedItem.is_vmad && <Badge variant="script" size="sm">VMAD</Badge>}
              {aliasResult?.has_mismatch && (
                <span className="editor-alias-warning" title={aliasResult.missing_in_trans.join(", ")}>
                  <AlertTriangle size={12} /> {t("editor.aliasMismatch")}
                </span>
              )}
            </div>
          </div>

          <div className="editor-dialog-main">
            <div className="editor-dialog-left">
              <div className="editor-source">
                <label>
                  {t("common.source")}
                  <button
                    className="editor-source-copy-btn"
                    onClick={() => navigator.clipboard.writeText(selectedItem.source)}
                    title={t("editor.copySourceTooltip", { defaultValue: "Copy source text" })}
                  >
                    <Copy size={12} />
                  </button>
                </label>
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

          {/* ── 拼写检查部分 ── */}
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

              {/* 建议面板 */}
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
                      onClick={() => handleIgnoreWord(spellResult.faults[selectedFaultIdx!].word)}
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
              </span>
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
