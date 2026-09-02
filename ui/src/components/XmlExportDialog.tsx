import React, { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Button, Modal } from "./ui";
import type { XmlExportScope } from "../api/strings";

export interface XmlExportDialogProps {
  open: boolean;
  onClose: () => void;
  /** 用户确认导出范围后回调 */
  onConfirm: (scope: XmlExportScope) => void;
  /** 目标文件路径（仅展示） */
  exportPath?: string;
  /** 当前多选数量（Selection 档提示用） */
  selectedCount?: number;
  /** 当前行总数 */
  totalCount?: number;
}

export const XmlExportDialog: React.FC<XmlExportDialogProps> = ({
  open,
  onClose,
  onConfirm,
  exportPath = "",
  selectedCount = 0,
  totalCount = 0,
}) => {
  const { t } = useTranslation();
  const [scope, setScope] = useState<XmlExportScope>("everything");

  // 打开时重置为默认 Everything（对齐 Delphi TFormXmlOpt 初始状态）
  useEffect(() => {
    if (open) {
      setScope("everything");
    }
  }, [open]);

  const handleConfirm = useCallback(() => {
    onConfirm(scope);
  }, [scope, onConfirm]);

  // Enter 确认，Esc 关闭
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
        e.preventDefault();
        handleConfirm();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, handleConfirm]);

  const scopeOptions: {
    value: XmlExportScope;
    label: string;
    hint?: string;
  }[] = [
    {
      value: "everything",
      label: t("xmlExport.scopeAll", "All (Eligible strings)"),
    },
    {
      value: "translated_and_validated",
      label: t("xmlExport.scopeTranslated", "Translated & Validated"),
    },
    {
      value: "selection",
      label: t("xmlExport.scopeSelection", "Selected Strings Only"),
      hint: `(${selectedCount} selected)`,
    },
    {
      value: "source_dest_diff",
      label: t("xmlExport.scopeDiff", "Source ≠ Dest (or Colab)"),
      hint: t("xmlExport.scopeDiffHint", "Colab ID set or hash differs"),
    },
  ];

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("xmlExport.title", "XML Export Options")}
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={handleConfirm}>
            {t("common.exportXml", "Export XML")}
          </Button>
        </>
      }
    >
      <div className="space-y-3">
        {exportPath && (
          <div className="text-xs text-muted-foreground truncate bg-muted/30 p-2 rounded border border-border/50">
            <span className="font-semibold text-foreground/80 mr-1">
              {t("xmlExport.file", "File")}:
            </span>
            {exportPath}
          </div>
        )}

        {/* 导出范围 (Delphi TFormXmlOpt.RadioGroup1) */}
        <div className="rounded-lg border border-border p-3 space-y-2 bg-card/40">
          <div className="font-semibold text-foreground flex items-center justify-between">
            <span>{t("xmlExport.range", "Export Range")}</span>
            <span className="text-xs font-normal text-muted-foreground">
              (RadioGroup1)
            </span>
          </div>
          <div className="grid grid-cols-1 gap-1.5 pt-1">
            {scopeOptions.map((opt) => (
              <label
                key={opt.value}
                className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors"
              >
                <input
                  type="radio"
                  name="xmlExportScope"
                  value={opt.value}
                  checked={scope === opt.value}
                  onChange={() => setScope(opt.value)}
                  className="text-primary focus:ring-primary h-4 w-4"
                />
                <span className="flex items-center justify-between flex-1">
                  <span>{opt.label}</span>
                  {opt.hint && (
                    <span className="text-xs text-muted-foreground ml-2">
                      {opt.hint}
                    </span>
                  )}
                </span>
              </label>
            ))}
          </div>
          <div className="text-xs text-muted-foreground pt-1">
            {t("xmlExport.totalHint", "Total strings: {{count}}", {
              count: totalCount,
            })}
          </div>
        </div>
      </div>
    </Modal>
  );
};
