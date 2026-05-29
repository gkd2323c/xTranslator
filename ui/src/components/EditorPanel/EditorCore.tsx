/**
 * EditorCore — 共享编辑逻辑 hook
 *
 * 将 EditorPanel 中的所有编辑状态和逻辑提取为可复用的 hook，
 * 供三种编辑器模式（Modal / Sidebar / Inline）共享使用。
 */

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAppStore, computeTranslationProgress } from "../../stores/appStore";
import {
  updateTranslation,
  heuristicSearch,
  translateString,
  setApiKey,
  checkAliases,
  spellCheckText,
  spellCheckSuggestions,
  spellCheckIgnore,
  type HeuristicMatchDTO,
  type AliasCheckResult,
  type SpellCheckResultDto,
} from "../../api/strings";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { replaceUtf8ByteRange } from "../../utils/utf8";

// ============================================================================
// 工具函数：HTML 转义和标签高亮
// ============================================================================

/**
 * 正则表达式：匹配 HTML 标签、$变量 和 {占位符}（用于语法高亮）
 */
const HIGHLIGHT_REGEX = /(<\/?[A-Za-z][^>]*>)|(\$\w+(?:\.\w+)*)|(\{[^}]+\})/g;

/**
 * 转义 HTML 特殊字符，防止 XSS 攻击
 */
function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/**
 * 语法高亮：区分标签类型颜色
 *
 *   - `<` 开头的 XML/HTML 标签 → 青色（.tag-highlight）
 *   - `$变量` → 紫色（.tag-variable）
 *   - `{占位符}` → 橙色（.tag-placeholder）
 *   - 普通文本 → 默认色
 *
 * 用途：在编辑器中显示源文本时，用不同颜色标记各类占位符
 */
export function highlightTags(text: string): string {
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
// useEditorCore Hook
// ============================================================================

/**
 * 核心编辑逻辑 hook
 *
 * 返回编辑器所需的全部状态、回调和派生值。
 * 各编辑模式组件仅负责渲染，逻辑由此 hook 统一管理。
 */
export function useEditorCore() {
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
   * 当选中的字符串改变时，重置编辑器状态，
   * 加载本地翻译文本，检查别名是否匹配。
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
   * Hook：防抖拼写检查（500ms）
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
   */
  const applyMatch = (translation: string) => {
    setLocalTrans(translation);
    toast.success(t("editor.translationCopied"));
  };

  // ========== 返回所有状态和函数 ==========
  return {
    // Store 订阅
    selectedItem,
    selectedId,
    language,
    targetLang,
    updateItemTranslation,
    setSelectedById,
    allItems,
    items,
    dataConfigs,

    // 翻译进度
    translationProgress,

    // 本地编辑
    localTrans,
    setLocalTrans,
    isSaving,
    handleSave,

    // 相似翻译搜索
    isSearching,
    matches,
    handleHeuristicSearch,

    // 机器翻译
    isTranslating,
    handleTranslate,

    // API 密钥
    showApiKeyDialog,
    setShowApiKeyDialog,
    apiKeyInput,
    setApiKeyInput,
    handleSetApiKey,

    // 别名检查
    aliasResult,

    // 拼写检查
    spellResult,
    selectedFaultIdx,
    suggestions,
    suggestionsLoading,
    handleSelectFault,
    handleApplySuggestion,
    handleIgnoreWord,

    // 字段大小警告
    fieldSizeWarning,

    // 导航和辅助
    jumpToUntranslated,
    applyMatch,

    // 国际化
    t,
  };
}
