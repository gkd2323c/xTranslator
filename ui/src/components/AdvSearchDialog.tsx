import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Modal, Button, Input } from "./ui";
import {
  AdvSearchState,
  AdvSearchCriteria,
  AdvSearchRegexFlags,
  AdvCompareMode,
  emptyAdvSearch,
  isAdvSearchEmpty,
  useAppStore,
} from "../stores/appStore";
import { Code2, Save, Trash2, List, Search } from "lucide-react";

// ============================================================================
// AdvSearchDialog — Advanced Search（DP-04）
// ============================================================================
//
// 对应 Delphi `TESVT_AdvSearch` 窗体。每个搜索维度独立生效（AND 关系）：
//   - Source      → 源文本
//   - Translated  → 译文
//   - EDID/FormID → EDID 文本子串，或 $/0x 前缀十六进制 FormID 精确匹配
//   - REC / FIELD → 记录签名 / 字段签名；REC 框支持 "REC:FIELD" 联合语法
//   - Keyword     → 保留维度（依赖 keyword 数据管道，当前提示不可用）
//
// Source/Translated/EDID/Keyword 各自可独立切换 Regex（对齐 sSearchUseRegex[1..4]）。
// 搜索 preset 可保存 / 载入 / 删除，持久化在 localStorage。

const PRESET_STORAGE_KEY = "xtranslator-advsearch-presets";

export interface AdvSearchPreset {
  name: string;
  state: AdvSearchState;
}

function loadPresets(): AdvSearchPreset[] {
  try {
    const raw = localStorage.getItem(PRESET_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (p): p is AdvSearchPreset =>
        p && typeof p.name === "string" && p.state && typeof p.state === "object"
    );
  } catch {
    return [];
  }
}

function savePresets(presets: AdvSearchPreset[]) {
  try {
    localStorage.setItem(PRESET_STORAGE_KEY, JSON.stringify(presets));
  } catch {
    // localStorage 不可用时静默失败（搜索 preset 不持久化）
  }
}

export interface AdvSearchDialogProps {
  open: boolean;
  onClose: () => void;
}

export const AdvSearchDialog: React.FC<AdvSearchDialogProps> = ({
  open,
  onClose,
}) => {
  const { t } = useTranslation();

  const setAdvSearch = useAppStore((s) => s.setAdvSearch);
  const clearAdvSearch = useAppStore((s) => s.clearAdvSearch);

  // 本地编辑状态：打开时从 store 快照，关闭时丢弃（仅在确认/即时应用时写入 store）
  const [draft, setDraft] = useState<AdvSearchState>(emptyAdvSearch());
  const [presets, setPresets] = useState<AdvSearchPreset[]>([]);
  const [presetName, setPresetName] = useState("");
  const [selectedPreset, setSelectedPreset] = useState<string | null>(null);
  const [searchActive, setSearchActive] = useState(false);

  // 打开时从 store 快照一次（仅 open 从 false→true 时同步，避免实时输入被 store 更新重置）
  const prevOpen = useRef(false);
  useEffect(() => {
    if (open && !prevOpen.current) {
      const current = useAppStore.getState().advSearch;
      setDraft(current ?? emptyAdvSearch());
      setSearchActive(current !== null);
      setPresets(loadPresets());
    }
    prevOpen.current = open;
  }, [open]);

  // 输入变化：更新 draft 并即时应用（对齐 Delphi ButtonedEdit1Change → launchSearchTimer）
  const updateCriteria = useCallback(
    (patch: Partial<AdvSearchCriteria>) => {
      setDraft((d) => {
        const next = { ...d, criteria: { ...d.criteria, ...patch } };
        if (searchActive) setAdvSearch(next);
        return next;
      });
    },
    [searchActive, setAdvSearch]
  );

  const updateRegex = useCallback(
    (key: keyof AdvSearchRegexFlags, value: boolean) => {
      setDraft((d) => {
        const next = { ...d, useRegex: { ...d.useRegex, [key]: value } };
        if (searchActive) setAdvSearch(next);
        return next;
      });
    },
    [searchActive, setAdvSearch]
  );

  const updateCompareMode = useCallback(
    (mode: AdvCompareMode) => {
      setDraft((d) => {
        const next = { ...d, compareMode: mode };
        if (searchActive) setAdvSearch(next);
        return next;
      });
    },
    [searchActive, setAdvSearch]
  );

  // 应用当前条件（不关闭面板）
  const applyNow = useCallback(() => {
    setAdvSearch(draft);
    setSearchActive(true);
  }, [draft, setAdvSearch]);

  // 关闭：如果用户从未应用过条件，则清空 store 中的激活状态
  const handleClose = useCallback(() => {
    if (!searchActive) {
      clearAdvSearch();
    }
    onClose();
  }, [searchActive, clearAdvSearch, onClose]);

  // 保存 preset
  const handleSavePreset = useCallback(() => {
    const name = presetName.trim();
    if (!name) return;
    setPresets((prev) => {
      const next = [
        ...prev.filter((p) => p.name !== name),
        { name, state: draft },
      ];
      savePresets(next);
      return next;
    });
    setPresetName("");
  }, [presetName, draft]);

  // 载入 preset
  const handleLoadPreset = useCallback(
    (name: string) => {
      const preset = presets.find((p) => p.name === name);
      if (!preset) return;
      setDraft(preset.state);
      setAdvSearch(preset.state);
      setSearchActive(true);
      setSelectedPreset(name);
    },
    [presets, setAdvSearch]
  );

  // 删除 preset
  const handleDeletePreset = useCallback(
    (name: string) => {
      setPresets((prev) => {
        const next = prev.filter((p) => p.name !== name);
        savePresets(next);
        return next;
      });
      if (selectedPreset === name) setSelectedPreset(null);
    },
    [selectedPreset]
  );

  // 清空所有条件
  const handleClearAll = useCallback(() => {
    const empty = emptyAdvSearch();
    setDraft(empty);
    setAdvSearch(empty);
  }, [setAdvSearch]);

  // Enter 应用（对齐 Delphi 行为：关闭时恢复简单搜索）
  const handleApplyAndClose = useCallback(() => {
    setAdvSearch(draft);
    setSearchActive(true);
    onClose();
  }, [draft, setAdvSearch, onClose]);

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        handleApplyAndClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, handleApplyAndClose]);

  if (!open) return null;

  const c = draft.criteria;
  const r = draft.useRegex;

  const empty = isAdvSearchEmpty(draft);

  // Regex 开关按钮组件（内联，避免重复 JSX）
  const regexToggle = (key: keyof AdvSearchRegexFlags, enabled: boolean) => (
    <Button
      variant="ghost"
      size="xs"
      icon={<Code2 size={12} />}
      active={enabled}
      onClick={() => updateRegex(key, !enabled)}
      title={t("advSearch.toggleRegex", { defaultValue: "Toggle regex" })}
      aria-label={t("advSearch.toggleRegex", { defaultValue: "Toggle regex" })}
    />
  );

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title={t("advSearch.title", { defaultValue: "Advanced Search" })}
      size="md"
      footer={
        <div className="flex items-center justify-between w-full">
          <div className="flex items-center gap-1">
            <Button size="sm" icon={<Search size={14} />} onClick={applyNow}>
              {t("advSearch.apply", { defaultValue: "Apply" })}
            </Button>
            <Button size="sm" onClick={handleApplyAndClose}>
              {t("advSearch.applyAndClose", { defaultValue: "Apply & Close" })}
            </Button>
            <Button size="sm" variant="ghost" onClick={handleClearAll} disabled={empty}>
              {t("advSearch.clear", { defaultValue: "Clear" })}
            </Button>
          </div>
          <Button size="sm" onClick={handleClose}>
            {t("common.close", { defaultValue: "Close" })}
          </Button>
        </div>
      }
    >
      <div className="space-y-3 p-1 text-sm">
        {/* 状态提示 */}
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            {searchActive
              ? t("advSearch.active", { defaultValue: "Advanced search is active. Simple search is suspended." })
              : t("advSearch.inactive", { defaultValue: "Click Apply to activate advanced search." })}
          </span>
          {!empty && (
            <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary font-medium">
              {t("advSearch.filtering", { defaultValue: "Filtering" })}
            </span>
          )}
        </div>

        {/* 六个搜索维度 */}
        <div className="rounded-lg border border-border p-3 space-y-2 bg-card/40">
          {/* Source */}
          <div className="flex items-center gap-2">
            <label className="w-24 shrink-0 text-xs font-semibold text-foreground/80">
              {t("advSearch.source", { defaultValue: "Source" })}
            </label>
            <div className="flex-1 flex items-center gap-1">
              <Input
                size="sm"
                className="font-mono"
                placeholder={t("advSearch.sourcePlaceholder", { defaultValue: "Search in source text…" })}
                value={c.source}
                onChange={(e) => updateCriteria({ source: e.target.value })}
              />
              {regexToggle("source", r.source)}
            </div>
          </div>

          {/* Translated */}
          <div className="flex items-center gap-2">
            <label className="w-24 shrink-0 text-xs font-semibold text-foreground/80">
              {t("advSearch.translated", { defaultValue: "Translated" })}
            </label>
            <div className="flex-1 flex items-center gap-1">
              <Input
                size="sm"
                className="font-mono"
                placeholder={t("advSearch.translatedPlaceholder", { defaultValue: "Search in translation…" })}
                value={c.translated}
                onChange={(e) => updateCriteria({ translated: e.target.value })}
              />
              {regexToggle("translated", r.translated)}
            </div>
          </div>

          {/* EDID/FormID */}
          <div className="flex items-center gap-2">
            <label className="w-24 shrink-0 text-xs font-semibold text-foreground/80">
              {t("advSearch.edid", { defaultValue: "EDID/FormID" })}
            </label>
            <div className="flex-1 flex items-center gap-1">
              <Input
                size="sm"
                className="font-mono"
                placeholder={t("advSearch.edidPlaceholder", { defaultValue: "EDID text or $00012345 / 0x00012345" })}
                value={c.edid}
                onChange={(e) => updateCriteria({ edid: e.target.value })}
              />
              {regexToggle("edid", r.edid)}
            </div>
          </div>

          {/* REC : FIELD 联合 */}
          <div className="flex items-center gap-2">
            <label className="w-24 shrink-0 text-xs font-semibold text-foreground/80">
              {t("advSearch.recField", { defaultValue: "REC : FIELD" })}
            </label>
            <div className="flex-1 flex items-center gap-1">
              <Input
                size="sm"
                className="font-mono flex-1"
                placeholder="INFO"
                value={c.rec}
                onChange={(e) => updateCriteria({ rec: e.target.value })}
                title={t("advSearch.recHint", { defaultValue: "Record signature, e.g. INFO; supports REC:FIELD" })}
              />
              <span className="text-muted-foreground text-xs">:</span>
              <Input
                size="sm"
                className="font-mono flex-1"
                placeholder="FULL"
                value={c.field}
                onChange={(e) => updateCriteria({ field: e.target.value })}
                title={t("advSearch.fieldHint", { defaultValue: "Field signature, e.g. FULL" })}
              />
            </div>
          </div>

          {/* Keyword */}
          <div className="flex items-center gap-2">
            <label className="w-24 shrink-0 text-xs font-semibold text-foreground/80">
              {t("advSearch.keyword", { defaultValue: "Keyword" })}
            </label>
            <div className="flex-1 flex items-center gap-1">
              <Input
                size="sm"
                className="font-mono"
                placeholder={t("advSearch.keywordPlaceholder", { defaultValue: "Record keyword (requires keyword data)" })}
                value={c.keyword}
                disabled
                onChange={(e) => updateCriteria({ keyword: e.target.value })}
              />
              {regexToggle("keyword", r.keyword)}
            </div>
            <span className="text-xs text-muted-foreground shrink-0" title={t("advSearch.keywordUnavailable", { defaultValue: "Keyword dictionary data is not yet available" })}>
              ⚠
            </span>
          </div>
        </div>

        {/* Source/Translated 比较模式 */}
        <div className="rounded-lg border border-border p-3 space-y-1.5 bg-card/40">
          <div className="text-xs font-semibold text-foreground/80">
            {t("advSearch.compareMode", { defaultValue: "Compare Source / Translated" })}
          </div>
          <div className="flex items-center gap-3 pt-0.5">
            {(
              [
                { mode: "any" as AdvCompareMode, label: "(.*) || (.*)" },
                { mode: "eq" as AdvCompareMode, label: "(.*) = (.*)" },
                { mode: "neq" as AdvCompareMode, label: "(.*) != (.*)" },
              ]
            ).map(({ mode, label }) => (
              <label key={mode} className="flex items-center gap-1.5 cursor-pointer font-mono text-xs">
                <input
                  type="radio"
                  name="compareMode"
                  checked={draft.compareMode === mode}
                  onChange={() => updateCompareMode(mode)}
                  className="text-primary focus:ring-primary h-3.5 w-3.5"
                />
                <span>{label}</span>
              </label>
            ))}
          </div>
        </div>

        {/* Presets */}
        <div className="rounded-lg border border-border p-3 space-y-2 bg-card/40">
          <div className="flex items-center justify-between">
            <div className="text-xs font-semibold text-foreground/80 flex items-center gap-1">
              <List size={13} />
              {t("advSearch.presets", { defaultValue: "Search Presets" })}
            </div>
          </div>

          {presets.length > 0 && (
            <div className="max-h-36 overflow-y-auto space-y-1">
              {presets.map((p) => (
                <div
                  key={p.name}
                  className={`flex items-center justify-between gap-2 px-2 py-1 rounded cursor-pointer hover:bg-muted/50 transition-colors text-xs ${
                    selectedPreset === p.name ? "bg-muted/60" : ""
                  }`}
                  onClick={() => handleLoadPreset(p.name)}
                >
                  <span className="truncate font-mono">{p.name}</span>
                  <Button
                    variant="ghost"
                    size="xs"
                    icon={<Trash2 size={12} />}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeletePreset(p.name);
                    }}
                    title={t("advSearch.deletePreset", { defaultValue: "Delete preset" })}
                    aria-label={t("advSearch.deletePreset", { defaultValue: "Delete preset" })}
                  />
                </div>
              ))}
            </div>
          )}

          <div className="flex items-center gap-1.5">
            <Input
              size="sm"
              placeholder={t("advSearch.presetName", { defaultValue: "Preset name…" })}
              value={presetName}
              onChange={(e) => setPresetName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  handleSavePreset();
                }
              }}
            />
            <Button size="sm" icon={<Save size={13} />} onClick={handleSavePreset} disabled={!presetName.trim()}>
              {t("advSearch.savePreset", { defaultValue: "Save" })}
            </Button>
          </div>
        </div>
      </div>
    </Modal>
  );
};

export default AdvSearchDialog;
