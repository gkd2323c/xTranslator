/**
 * EditorInline — 内联模式编辑器
 *
 * 在选中表格行下方渲染的紧凑编辑器（~120px 高度）。
 * 使用 useEditorCore() hook 获取所有状态和处理器。
 * 布局：左侧源文本 → 中间翻译输入 → 右侧操作按钮。
 */

import { useEditorCore, highlightTags } from "./EditorCore";
import { useAppStore } from "../../stores/appStore";
import { useTranslation } from "react-i18next";
import { Button, Textarea, Badge } from "../ui";
import { Save, ArrowUp, ArrowDown, Search, Languages } from "lucide-react";
import type { EditorPanelProps } from "./index";

export function EditorInline({ open, onClose }: EditorPanelProps) {
  const { t } = useTranslation();
  const selectedItem = useAppStore((s) => s.selectedItem);
  const core = useEditorCore();

  if (!open || !selectedItem) return null;

  return (
    <div className="editor-inline">
      <div className="editor-inline-row">
        {/* 源文本（只读） */}
        <div
          className="editor-inline-source"
          dangerouslySetInnerHTML={{ __html: highlightTags(selectedItem.source) }}
        />
        {/* 翻译输入框 */}
        <Textarea
          value={core.localTrans}
          onChange={(e) => core.setLocalTrans(e.target.value)}
          onKeyDown={(e) => {
            if (e.ctrlKey && e.key === "Enter") core.handleSave();
            if (e.key === "Escape") onClose();
          }}
          rows={3}
          className="editor-inline-textarea"
          placeholder={t("editor.enterTranslation")}
          autoFocus
        />
        {/* 操作按钮 */}
        <div className="editor-inline-actions">
          <Button
            size="xs"
            onClick={core.handleSave}
            loading={core.isSaving}
            icon={<Save size={12} />}
          >
            {t("editor.save")}
          </Button>
          <Button
            size="xs"
            variant="ghost"
            onClick={core.handleHeuristicSearch}
            disabled={core.isSearching}
          >
            <Search size={12} />
          </Button>
          <Button
            size="xs"
            variant="ghost"
            onClick={core.handleTranslate}
            disabled={core.isTranslating}
          >
            <Languages size={12} />
          </Button>
          <Button
            size="xs"
            variant="ghost"
            onClick={() => core.jumpToUntranslated("prev")}
          >
            <ArrowUp size={12} />
          </Button>
          <Button
            size="xs"
            variant="ghost"
            onClick={() => core.jumpToUntranslated("next")}
          >
            <ArrowDown size={12} />
          </Button>
          <Badge
            variant={selectedItem.status === "translated" ? "translated" : selectedItem.status === "locked" ? "locked" : "incomplete"}
            size="sm"
          >
            {selectedItem.status}
          </Badge>
          <span className="editor-inline-char-count">
            {core.localTrans.length} chars
          </span>
        </div>
      </div>
    </div>
  );
}
