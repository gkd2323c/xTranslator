/**
 * EditorSidebar — 侧边栏模式编辑器
 *
 * 以固定右侧面板（40% 宽度）渲染，而非弹窗模式。
 * 适用于连续翻译场景，无需反复打开/关闭弹窗。
 * 使用 useEditorCore() hook 共享所有编辑逻辑。
 */

import { useEffect } from "react";
import { useEditorCore, highlightTags } from "./EditorCore";
import { useAppStore } from "../../stores/appStore";
import { useTranslation } from "react-i18next";
import { Button, Textarea, Badge, ProgressBar } from "../ui";
import { Save, Search, Languages, ArrowRight, ArrowUp, ArrowDown, Sparkles, X } from "lucide-react";
import type { EditorPanelProps } from "./index";

/**
 * EditorSidebar 组件
 *
 * 布局结构（自上而下）：
 *   1. 标题栏（ID + 签名 + 关闭按钮）
 *   2. 元数据行（FormID / Record / Field / Status / VMAD）
 *   3. 源文本区（带语法高亮）
 *   4. 翻译输入区（Textarea）
 *   5. 操作按钮栏（保存 / 搜索 / 翻译 / 导航）
 *   6. 相似翻译列表（如有）
 *   7. 拼写检查区（如有）
 *   8. 底部栏（进度条 + 字数统计）
 */
export function EditorSidebar({ open, onClose }: EditorPanelProps) {
  const { t } = useTranslation();
  const selectedItem = useAppStore((s) => s.selectedItem);
  const core = useEditorCore();

  // ========== 全局键盘快捷键 ==========

  /**
   * 侧边栏打开时激活的快捷键
   *   - Ctrl+↓：跳转下一个未翻译项
   *   - Ctrl+↑：跳转上一个未翻译项
   *   - Ctrl+H：相似翻译搜索
   *   - Ctrl+T：机器翻译
   */
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "ArrowDown") {
        e.preventDefault();
        core.jumpToUntranslated("next");
      }
      if (e.ctrlKey && e.key === "ArrowUp") {
        e.preventDefault();
        core.jumpToUntranslated("prev");
      }
      if (e.ctrlKey && e.key === "h" && selectedItem) {
        e.preventDefault();
        core.handleHeuristicSearch();
      }
      if (e.ctrlKey && e.key === "t" && selectedItem) {
        e.preventDefault();
        core.handleTranslate();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, selectedItem, core.jumpToUntranslated, core.handleHeuristicSearch, core.handleTranslate]);

  /**
   * 编辑框内本地键盘事件
   *   - Ctrl+Enter：保存翻译
   */
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.ctrlKey && e.key === "Enter") {
      core.handleSave();
    }
  };

  if (!open || !selectedItem) return null;

  return (
    <div className="editor-sidebar">
      {/* ── 标题栏 ── */}
      <div className="editor-sidebar-header">
        <span className="editor-sidebar-title">
          #{selectedItem.id} {selectedItem.record_sig}:{selectedItem.field_sig}
        </span>
        <button className="editor-sidebar-close" onClick={onClose}>
          <X size={16} />
        </button>
      </div>

      {/* ── 元数据行 ── */}
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
        <Badge
          variant={
            selectedItem.status === "translated"
              ? "translated"
              : selectedItem.status === "incomplete"
                ? "incomplete"
                : "locked"
          }
        >
          {selectedItem.status}
        </Badge>
        {selectedItem.is_vmad && <Badge variant="script" size="sm">VMAD</Badge>}
      </div>

      {/* ── 源文本区 ── */}
      <div className="editor-source">
        <label>{t("common.source")}</label>
        <div
          className="editor-source-text"
          dangerouslySetInnerHTML={{ __html: highlightTags(selectedItem.source) }}
        />
      </div>

      {/* ── 翻译输入区 ── */}
      <div className="editor-translation">
        <label>{t("common.translation")}</label>
        <Textarea
          value={core.localTrans}
          onChange={(e) => core.setLocalTrans(e.target.value)}
          onKeyDown={handleKeyDown}
          rows={6}
          className="editor-textarea"
          placeholder={t("editor.enterTranslation")}
          autoFocus
        />
      </div>

      {/* ── 操作按钮栏 ── */}
      <div className="editor-sidebar-actions">
        <Button
          size="sm"
          onClick={core.handleSave}
          loading={core.isSaving}
          icon={<Save size={14} />}
        >
          {t("editor.save")}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={core.handleHeuristicSearch}
          disabled={core.isSearching}
        >
          <Search size={14} />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={core.handleTranslate}
          disabled={core.isTranslating}
        >
          <Languages size={14} />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => core.setLocalTrans(selectedItem.source)}
          title={t("editor.copySourceTooltip", { defaultValue: "Copy Source" })}
        >
          <ArrowRight size={14} />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => core.jumpToUntranslated("prev")}
          title="Ctrl+ArrowUp"
        >
          <ArrowUp size={14} />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => core.jumpToUntranslated("next")}
          title="Ctrl+ArrowDown"
        >
          <ArrowDown size={14} />
        </Button>
      </div>

      {/* ── 相似翻译列表 ── */}
      {core.matches.length > 0 && (
        <div className="editor-matches">
          <label>{t("editor.similarTranslations")}</label>
          <div className="matches-list">
            {core.matches.map((m, i) => (
              <div key={i} className="match-item" onClick={() => core.applyMatch(m.translation)}>
                <div className="match-source" title={m.source}>{m.source}</div>
                <div className="match-translation">{m.translation}</div>
                <div className="match-meta">
                  <span className="match-sim">{(m.similarity * 100).toFixed(0)}%</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ── 拼写检查区 ── */}
      {core.spellResult && core.spellResult.active && core.spellResult.faults.length > 0 && (
        <div className="editor-spellcheck">
          <div className="spellcheck-summary">
            <Sparkles size={14} />
            <span>{core.spellResult.faults.length} misspelled word(s)</span>
          </div>
          <div className="spellcheck-faults">
            {core.spellResult.faults.map((fault, idx) => (
              <button
                key={`${fault.word}-${idx}`}
                className={`spellcheck-chip ${core.selectedFaultIdx === idx ? "spellcheck-chip-selected" : ""}`}
                onClick={() => core.handleSelectFault(idx)}
              >
                {fault.word}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* ── 底部栏 ── */}
      <div className="editor-sidebar-footer">
        <ProgressBar
          value={core.translationProgress.translated}
          max={core.translationProgress.total}
          variant="gradient"
          size="sm"
          showLabel
          label={t("sidebar.progress")}
        />
        <span className="editor-char-count">
          {t("editor.sourceChars")}: {selectedItem.source.length} | {t("editor.transChars")}: {core.localTrans.length}
        </span>
      </div>
    </div>
  );
}
