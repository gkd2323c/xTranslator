import React, { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Modal } from "./ui";
import type {
  SstOverwriteScope,
  SstMatchMode,
  SstApplyOptions,
} from "../api/strings";

export interface ApplySstDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: (options: SstApplyOptions) => void;
  selectedCount?: number;
  filteredCount?: number;
  sstPath?: string | null;
}

export const ApplySstDialog: React.FC<ApplySstDialogProps> = ({
  open,
  onClose,
  onConfirm,
  selectedCount = 0,
  filteredCount = 0,
  sstPath = "",
}) => {
  const { t } = useTranslation();

  const [overwriteScope, setOverwriteScope] =
    useState<SstOverwriteScope>("all");
  const [matchMode, setMatchMode] =
    useState<SstMatchMode>("form_id_strict_string");
  const [tagOnly, setTagOnly] = useState<boolean>(false);
  const [resetState, setResetState] = useState<boolean>(false);
  const [restrictToFilter, setRestrictToFilter] = useState<boolean>(false);

  // 默认快捷键 Esc 关闭，Enter 应用
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
        e.preventDefault();
        handleApply();
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, overwriteScope, matchMode, tagOnly, resetState, restrictToFilter, onClose]);

  const handleApply = useCallback(() => {
    onConfirm({
      overwrite_scope: overwriteScope,
      match_mode: matchMode,
      tag_only: tagOnly,
      reset_state: resetState,
      restrict_to_filter: restrictToFilter,
    });
    onClose();
  }, [
    overwriteScope,
    matchMode,
    tagOnly,
    resetState,
    restrictToFilter,
    onConfirm,
    onClose,
  ]);

  if (!open) return null;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("applySst.title", "Apply SST Options (Delphi Parity)")}
      size="md"
    >
      <div className="space-y-4 p-2 text-sm">
        {sstPath && (
          <div className="text-xs text-muted-foreground truncate bg-muted/30 p-2 rounded border border-border/50">
            <span className="font-semibold text-foreground/80 mr-1">
              {t("applySst.file", "SST File")}:
            </span>
            {sstPath}
          </div>
        )}

        {/* 覆盖范围 (5 种 Overwrite Scope) */}
        <div className="rounded-lg border border-border p-3 space-y-2 bg-card/40">
          <div className="font-semibold text-foreground flex items-center justify-between">
            <span>{t("applySst.overwriteScope", "Overwrite Range")}</span>
            <span className="text-xs font-normal text-muted-foreground">
              (RadioGroup1)
            </span>
          </div>
          <div className="grid grid-cols-1 gap-1.5 pt-1">
            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="overwriteScope"
                value="all"
                checked={overwriteScope === "all"}
                onChange={() => setOverwriteScope("all")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>{t("applySst.scopeAll", "All (Everything unlocked)")}</span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="overwriteScope"
                value="no_trans_exclusive"
                checked={overwriteScope === "no_trans_exclusive"}
                onChange={() => setOverwriteScope("no_trans_exclusive")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t("applySst.scopeNoTrans", "NoTrans (Untranslated exclusive)")}
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="overwriteScope"
                value="no_trans_and_partial"
                checked={overwriteScope === "no_trans_and_partial"}
                onChange={() => setOverwriteScope("no_trans_and_partial")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t(
                  "applySst.scopeNoTransAndPartial",
                  "Strictly Untranslated (Exclude Partial)"
                )}
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="overwriteScope"
                value="partial_only"
                checked={overwriteScope === "partial_only"}
                onChange={() => setOverwriteScope("partial_only")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t("applySst.scopePartialOnly", "Partial String Only")}
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="overwriteScope"
                value="selection"
                checked={overwriteScope === "selection"}
                onChange={() => setOverwriteScope("selection")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span className="flex items-center justify-between flex-1">
                <span>{t("applySst.scopeSelection", "Selected Strings Only")}</span>
                <span className="text-xs text-muted-foreground ml-2">
                  ({selectedCount} selected)
                </span>
              </span>
            </label>
          </div>
        </div>

        {/* 匹配模式 (4 种 Match Mode) */}
        <div className="rounded-lg border border-border p-3 space-y-2 bg-card/40">
          <div className="font-semibold text-foreground flex items-center justify-between">
            <span>{t("applySst.matchMode", "Matching Method")}</span>
            <span className="text-xs font-normal text-muted-foreground">
              (RadioGroup2)
            </span>
          </div>
          <div className="grid grid-cols-1 gap-1.5 pt-1">
            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="matchMode"
                value="form_id_only"
                checked={matchMode === "form_id_only"}
                onChange={() => setMatchMode("form_id_only")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t(
                  "applySst.modeFormIdOnly",
                  "FORMID Only (FORMID/EDID + Field + Index)"
                )}
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="matchMode"
                value="form_id_strict_string"
                checked={matchMode === "form_id_strict_string"}
                onChange={() => setMatchMode("form_id_strict_string")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t(
                  "applySst.modeStrictString",
                  "FORMID + Strict String Control (Exact text + Index)"
                )}
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="matchMode"
                value="form_id_relaxed_string"
                checked={matchMode === "form_id_relaxed_string"}
                onChange={() => setMatchMode("form_id_relaxed_string")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t(
                  "applySst.modeRelaxedString",
                  "FORMID + Relaxed String Control (Exact text, ignore Index)"
                )}
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="radio"
                name="matchMode"
                value="string_only"
                checked={matchMode === "string_only"}
                onChange={() => setMatchMode("string_only")}
                className="text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t(
                  "applySst.modeStringOnly",
                  "String Only (Ignore FormID, exact source text)"
                )}
              </span>
            </label>
          </div>
        </div>

        {/* 附加标志 (3 个 CheckBox) */}
        <div className="rounded-lg border border-border p-3 space-y-2 bg-card/40">
          <div className="font-semibold text-foreground">
            {t("applySst.additionalOptions", "Additional Flags")}
          </div>
          <div className="space-y-2 pt-1">
            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="checkbox"
                checked={tagOnly}
                onChange={(e) => setTagOnly(e.target.checked)}
                className="rounded border-border text-primary focus:ring-primary h-4 w-4"
              />
              <span className="flex items-center justify-between flex-1">
                <span>{t("applySst.tagOnly", "Apply Tag Only (Do not overwrite text)")}</span>
                <span className="text-xs text-muted-foreground ml-2">(F2 Tag)</span>
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="checkbox"
                checked={resetState}
                onChange={(e) => setResetState(e.target.checked)}
                className="rounded border-border text-primary focus:ring-primary h-4 w-4"
              />
              <span>
                {t(
                  "applySst.resetState",
                  "Reset StringState Before Match (Also resets unmatched eligible rows)"
                )}
              </span>
            </label>

            <label className="flex items-center space-x-2 cursor-pointer hover:bg-muted/40 p-1 rounded transition-colors">
              <input
                type="checkbox"
                checked={restrictToFilter}
                onChange={(e) => setRestrictToFilter(e.target.checked)}
                className="rounded border-border text-primary focus:ring-primary h-4 w-4"
              />
              <span className="flex items-center justify-between flex-1">
                <span>
                  {t(
                    "applySst.restrictToFilter",
                    "Restrict to Current Filter (Visible rows only)"
                  )}
                </span>
                {filteredCount > 0 && (
                  <span className="text-xs text-muted-foreground ml-2">
                    ({filteredCount} visible)
                  </span>
                )}
              </span>
            </label>
          </div>
        </div>

        {/* 底部按钮栏 */}
        <div className="flex justify-end space-x-2 pt-2 border-t border-border mt-3">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-1.5 text-sm font-medium rounded-md border border-border hover:bg-muted transition-colors"
          >
            {t("common.cancel", "Cancel")}
          </button>
          <button
            type="button"
            onClick={handleApply}
            className="px-4 py-1.5 text-sm font-medium rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors shadow-sm"
          >
            {t("common.apply", "Apply")} (Enter)
          </button>
        </div>
      </div>
    </Modal>
  );
};
