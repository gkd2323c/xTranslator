import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAppStore, computeTranslationProgress } from "../stores/appStore";
import { updateTranslation, heuristicSearch, translateString, setApiKey, tcscConvert, rtlReverse, shapeArabic, deshapeArabic, checkAliases, spellCheckText, spellCheckSuggestions, spellCheckIgnore, type HeuristicMatchDTO, type AliasCheckResult, type SpellCheckResultDto, type SpellFaultDto } from "../api/strings";
import { Save, Search, Languages, Key, AlertTriangle, ArrowRight, Copy, ArrowUp, ArrowDown, Sparkles } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { Button, Textarea, Badge, Modal, Input, ProgressBar } from "./ui";
import { replaceUtf8ByteRange } from "../utils/utf8";

// ============================================================================
// EditorDialog 组件 - 字符串翻译编辑器
// ============================================================================
//
// 职责：
//   - 提供单个字符串的编辑界面
//   - 支持多种翻译辅助功能（相似翻译、机器翻译、拼写检查）
//   - 处理字符串的保存和验证
//   - 支持文本转换（简繁转换、RTL、阿拉伯文形状）
//
// 核心功能：
//   1. 本地编辑：编辑翻译文本，支持 Ctrl+Enter 保存
//   2. 相似翻译搜索：基于启发式算法找到相似的已翻译字符串
//   3. 机器翻译：调用翻译 API 自动翻译
//   4. 拼写检查：实时检查拼写错误并提供建议
//   5. 别名检查：验证翻译中的别名是否与源文本匹配
//   6. 字段大小验证：检查翻译是否超过字段大小限制
//   7. 文本转换：简繁转换、RTL 反向、阿拉伯文形状处理
//
// 键盘快捷键：
//   - Ctrl+Enter：保存翻译
//   - Ctrl+↑：跳转到上一个未翻译项
//   - Ctrl+↓：跳转到下一个未翻译项
//   - Ctrl+H：启动相似翻译搜索
//   - Ctrl+T：启动机器翻译
//
// 状态管理：
//   - localTrans：本地编辑的翻译文本
//   - matches：相似翻译搜索结果
//   - spellResult：拼写检查结果
//   - aliasResult：别名检查结果
//   - fieldSizeWarning：字段大小警告
//
// ============================================================================

// 正则表达式：匹配 HTML 标签、$变量 和 {占位符}（用于语法高亮）
const HIGHLIGHT_REGEX = /(<\/?[A-Za-z][^>]*>)|(\$\w+(?:\.\w+)*)|(\{[^}]+\})/g;

// ============================================================================
// 工具函数：HTML 转义和标签高亮
// ============================================================================

/**
 * 转义 HTML 特殊字符，防止 XSS 攻击
 * 
 * @param s - 原始字符串
 * @returns 转义后的 HTML 安全字符串
 */
function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/**
 * 语法高亮：区分标签类型颜色
 * 
 * 功能：
 *   - `<` 开头的 XML/HTML 标签 → 青色（.tag-highlight）
 *   - `$变量` → 紫色（.tag-variable）
 *   - `{占位符}` → 橙色（.tag-placeholder）
 *   - 普通文本 → 默认色
 *   - 转义所有文本内容防止 XSS
 * 
 * 用途：在编辑器中显示源文本时，用不同颜色标记各类占位符
 * 
 * @param text - 包含 HTML 标签或占位符的文本
 * @returns HTML 字符串，各类标签被对应颜色的 <span> 包装
 */
function highlightTags(text: string): string {
  let lastIndex = 0;
  const parts: string[] = [];
  let match: RegExpExecArray | null;

  while ((match = HIGHLIGHT_REGEX.exec(text)) !== null) {
    // 添加匹配前的普通文本
    if (match.index > lastIndex) {
      parts.push(escapeHtml(text.slice(lastIndex, match.index)));
    }

    if (match[1]) {
      // Group 1: XML/HTML 标签 → 青色
      parts.push(`<span class="tag-highlight">${escapeHtml(match[1])}</span>`);
    } else if (match[2]) {
      // Group 2: $变量 → 紫色
      parts.push(`<span class="tag-variable">${escapeHtml(match[2])}</span>`);
    } else if (match[3]) {
      // Group 3: {占位符} → 橙色
      parts.push(`<span class="tag-placeholder">${escapeHtml(match[3])}</span>`);
    }

    lastIndex = match.index + match[0].length;
  }

  // 添加剩余文本
  if (lastIndex < text.length) {
    parts.push(escapeHtml(text.slice(lastIndex)));
  }

  return parts.join("");
}

// ============================================================================
// EditorDialog 组件 Props 接口
// ============================================================================

export interface EditorDialogProps {
  open: boolean;           // 对话框是否打开
  onClose: () => void;     // 关闭对话框的回调
}

export function EditorDialog({ open, onClose }: EditorDialogProps) {
  // ========== 国际化和 Store 订阅 ==========
  const { t } = useTranslation();
  
  // 当前选中的字符串项
  const selectedItem = useAppStore((s) => s.selectedItem);
  const selectedId = useAppStore((s) => s.selectedId);
  
  // 语言设置
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  
  // Store 操作函数
  const updateItemTranslation = useAppStore((s) => s.updateItemTranslation);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  
  // 数据集
  const allItems = useAppStore((s) => s.allItems);
  const items = useAppStore((s) => s.items);
  const dataConfigs = useAppStore((s) => s.dataConfigs);

  // 计算翻译进度（已翻译/总数）
  const translationProgress = useMemo(() => computeTranslationProgress(allItems), [allItems]);

  // ========== 本地编辑状态 ==========
  const [localTrans, setLocalTrans] = useState("");  // 本地编辑的翻译文本
  const [isSaving, setIsSaving] = useState(false);   // 保存中标志
  
  // ========== 相似翻译搜索状态 ==========
  const [isSearching, setIsSearching] = useState(false);
  const [matches, setMatches] = useState<HeuristicMatchDTO[]>([]);
  
  // ========== 机器翻译状态 ==========
  const [isTranslating, setIsTranslating] = useState(false);
  const [showApiKeyDialog, setShowApiKeyDialog] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");

  // ========== 别名检查状态 ==========
  const [aliasResult, setAliasResult] = useState<AliasCheckResult | null>(null);

  // ========== 拼写检查状态 ==========
  const [spellResult, setSpellResult] = useState<SpellCheckResultDto | null>(null);
  const [selectedFaultIdx, setSelectedFaultIdx] = useState<number | null>(null);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [suggestionsLoading, setSuggestionsLoading] = useState(false);
  const spellTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ========== 字段大小验证 ==========
  /**
   * 计算字段大小警告
   * 
   * 功能：
   *   - 检查翻译文本的字节长度
   *   - 与字段大小限制比较
   *   - 如果超过限制，返回警告信息
   * 
   * 依赖：
   *   - selectedItem：当前选中的字符串
   *   - dataConfigs：字段大小配置
   *   - localTrans：本地翻译文本
   */
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

  // ========== 核心功能函数 ==========

  /**
   * 跳转到下一个/上一个未翻译的字符串
   * 
   * 功能：
   *   - 从当前选中的字符串开始搜索
   *   - 按指定方向（next/prev）查找未翻译的项
   *   - 如果找到，选中该项
   *   - 如果没有找到，显示提示信息
   * 
   * @param direction - 搜索方向："next" 或 "prev"
   */
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

  /**
   * Hook：初始化编辑器状态
   * 
   * 功能：
   *   - 当选中的字符串改变时，重置编辑器状态
   *   - 加载本地翻译文本
   *   - 清空搜索结果和拼写检查结果
   *   - 检查别名是否匹配
   * 
   * 依赖：selectedId
   */
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

  // ========== 拼写检查功能 ==========

  /**
   * 执行拼写检查
   * 
   * 功能：
   *   - 调用后端拼写检查 API
   *   - 如果有错误，保存结果
   *   - 如果没有错误但拼写检查启用，显示"无错误"状态
   * 
   * @param text - 要检查的文本
   */
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
        // 显示"无错误"状态
        setSpellResult(result);
      } else {
        setSpellResult(null);
      }
    } catch {
      // 静默失败
    }
  }, []);

  /**
   * Hook：防抖拼写检查
   * 
   * 功能：
   *   - 当本地翻译文本改变时，延迟 500ms 后执行拼写检查
   *   - 避免频繁调用 API
   *   - 组件卸载时清理定时器
   * 
   * 依赖：localTrans, doSpellCheck
   */
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

  /**
   * 选中拼写错误并获取建议
   * 
   * 功能：
   *   - 标记选中的错误
   *   - 调用 API 获取拼写建议
   *   - 显示建议列表
   * 
   * @param index - 错误在 faults 数组中的索引
   */
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

  /**
   * 应用拼写建议
   * 
   * 功能：
   *   - 将选中的建议替换到翻译文本中
   *   - 替换范围由错误的 start_byte 和 end_byte 确定
   *   - 清空建议列表
   * 
   * @param suggestion - 要应用的建议文本
   */
  const handleApplySuggestion = useCallback((suggestion: string) => {
    if (selectedFaultIdx === null || !spellResult) return;
    const fault = spellResult.faults[selectedFaultIdx];
    const newText = replaceUtf8ByteRange(
      localTrans,
      fault.start_byte,
      fault.end_byte,
      suggestion,
    );
    setLocalTrans(newText);
    setSelectedFaultIdx(null);
    setSuggestions([]);
  }, [selectedFaultIdx, spellResult, localTrans]);

  /**
   * 忽略拼写错误
   * 
   * 功能：
   *   - 将错误的单词添加到忽略列表
   *   - 从当前拼写检查结果中移除该错误
   *   - 显示成功提示
   * 
   * @param word - 要忽略的单词
   */
  const handleIgnoreWord = useCallback(async (word: string) => {
    try {
      await spellCheckIgnore(word);
      // 从错误列表中移除
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

  // ========== 翻译保存和搜索功能 ==========

  /**
   * 保存翻译
   * 
   * 功能：
   *   - 调用后端 API 保存翻译
   *   - 更新本地 store 中的翻译
   *   - 显示成功或失败提示
   */
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

  /**
   * 启发式搜索相似翻译
   * 
   * 功能：
   *   - 基于源文本的相似度搜索已翻译的字符串
   *   - 返回最相似的 5 个翻译
   *   - 相似度阈值为 0.4（40%）
   * 
   * 用途：帮助翻译者找到相似的已翻译字符串作为参考
   */
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

  /**
   * 机器翻译
   * 
   * 功能：
   *   - 调用翻译 API 自动翻译源文本
   *   - 将结果填充到翻译文本框
   *   - 如果需要 API 密钥，显示设置对话框
   */
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

  /**
   * 设置翻译 API 密钥
   * 
   * 功能：
   *   - 验证 API 密钥不为空
   *   - 调用后端 API 保存密钥
   *   - 关闭设置对话框
   */
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

  /**
   * 应用相似翻译
   * 
   * 功能：
   *   - 将选中的相似翻译复制到编辑框
   *   - 显示成功提示
   * 
   * @param translation - 要应用的翻译文本
   */
  const applyMatch = (translation: string) => {
    setLocalTrans(translation);
    toast.success(t("editor.translationCopied"));
  };

  // ========== 键盘事件处理 ==========

  /**
   * Hook：全局键盘快捷键
   * 
   * 支持的快捷键：
   *   - Ctrl+↓：跳转到下一个未翻译项
   *   - Ctrl+↑：跳转到上一个未翻译项
   *   - Ctrl+H：启动相似翻译搜索
   *   - Ctrl+T：启动机器翻译
   * 
   * 依赖：open, selectedItem, jumpToUntranslated, handleHeuristicSearch, handleTranslate
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
   * 本地键盘事件处理（在编辑框内）
   * 
   * 支持的快捷键：
   *   - Ctrl+Enter：保存翻译
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
      // Restore cursor position after state update
      requestAnimationFrame(() => {
        textarea.selectionStart = textarea.selectionEnd = start + 2;
      });
    }
  };

  // ========== 渲染 ==========

  // 对话框标题：显示字符串 ID 和 EDID
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
