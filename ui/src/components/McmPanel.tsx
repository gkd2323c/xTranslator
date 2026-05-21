import { useState, useCallback, useMemo } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { FileText, FileUp, Save, RotateCcw, Copy, X } from "lucide-react";
import toast from "react-hot-toast";
import { loadMcmFile, saveMcmFile, mcmCompare } from "../api/strings";
import type { McmFileDto, McmEntryDto, McmComparePolicy, McmCompareRequest, McmCompareResult } from "../api/strings";
import { Button, Badge, EmptyState, Modal, Select } from "./ui";

// ── 翻译状态判定 ────────────────────────────────────────────────────

type EntryStatus = "translated" | "partial" | "untranslated";

function getEntryStatus(entry: McmEntryDto): EntryStatus {
  if (entry.translation.length === 0) return "untranslated";
  if (entry.translation === entry.source) return "untranslated";
  const ratio = entry.translation.length / Math.max(entry.source.length, 1);
  if (ratio < 0.3) return "partial";
  return "translated";
}

const STATUS_CONFIG: Record<EntryStatus, { labelKey: string; color: string; icon: string }> = {
  translated:    { labelKey: "mcm.statusTranslated",    color: "#22c55e", icon: "✓" },
  partial:       { labelKey: "mcm.statusPartial",       color: "#f59e0b", icon: "◐" },
  untranslated:  { labelKey: "mcm.statusUntranslated",  color: "#6b7280", icon: "○" },
};

// ── Compare Result Dialog ──────────────────────────────────────────

interface CompareResultDialogProps {
  result: McmCompareResult;
  entries: McmEntryDto[];
  onClose: () => void;
}

function CompareResultDialog({ result, entries, onClose }: CompareResultDialogProps) {
  const { t } = useTranslation();

  const updatedEntryMap = useMemo(
    () => new Map(result.updated_entries.map((e) => [e.line_index, e])),
    [result.updated_entries]
  );

  // 显示所有发生变化的条目（source + 旧译文 + 新译文）
  const changedEntries = useMemo(() => {
    return entries
      .filter((e) => updatedEntryMap.has(e.line_index))
      .map((e) => ({
        ...e,
        newTranslation: updatedEntryMap.get(e.line_index)!.translation,
      }));
  }, [entries, updatedEntryMap]);

  return (
    <Modal
      open
      onClose={onClose}
      title={t("mcm.compareResults", { defaultValue: "Compare Results" })}
      size="lg"
      footer={
        <Button variant="ghost" onClick={onClose}>
          {t("common.close")}
        </Button>
      }
    >
      <div className="mcm-compare-result">
        {/* Summary stats */}
        <div className="mcm-compare-summary">
          <div className="mcm-compare-stat">
            <span className="mcm-compare-stat-value" style={{ color: "#22c55e" }}>{result.matched}</span>
            <span className="mcm-compare-stat-label">{t("mcm.matched", { defaultValue: "Matched" })}</span>
          </div>
          <div className="mcm-compare-stat">
            <span className="mcm-compare-stat-value" style={{ color: "#ef4444" }}>{result.unmatched}</span>
            <span className="mcm-compare-stat-label">{t("mcm.unmatched", { defaultValue: "Unmatched" })}</span>
          </div>
          <div className="mcm-compare-stat">
            <span className="mcm-compare-stat-value" style={{ color: "#3b82f6" }}>{result.updated_entries.length}</span>
            <span className="mcm-compare-stat-label">{t("mcm.updatedCount", { defaultValue: "Updated" })}</span>
          </div>
        </div>

        {/* Changed entries list */}
        {changedEntries.length > 0 && (
          <div className="mcm-compare-changes">
            <h4>{t("mcm.changesList", { defaultValue: "Changes applied" })}</h4>
            <div className="mcm-compare-change-list" style={{ maxHeight: 300, overflowY: "auto" }}>
              {changedEntries.map((entry) => (
                <div key={entry.line_index} className="mcm-compare-change-row">
                  <div className="mcm-compare-change-id">{entry.id}</div>
                  <div className="mcm-compare-change-source">{entry.source}</div>
                  <div className="mcm-compare-change-fields">
                    <div className="mcm-compare-change-old">
                      <span className="mcm-compare-change-label">{t("mcm.oldTranslation", { defaultValue: "Old" })}:</span>
                      <span className="mcm-compare-change-text">{entry.translation || "—"}</span>
                    </div>
                    <div className="mcm-compare-change-arrow">→</div>
                    <div className="mcm-compare-change-new">
                      <span className="mcm-compare-change-label">{t("mcm.newTranslation", { defaultValue: "New" })}:</span>
                      <span className="mcm-compare-change-text mcm-diff-added">{entry.newTranslation}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {changedEntries.length === 0 && (
          <div className="sidepanel-hint" style={{ textAlign: "center", padding: 16 }}>
            {t("mcm.noChanges", { defaultValue: "No entries were updated" })}
          </div>
        )}
      </div>
    </Modal>
  );
}

// ── Main Component ─────────────────────────────────────────────────

export function McmPanel() {
  const { t } = useTranslation();
  const [file, setFile] = useState<McmFileDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [modified, setModified] = useState(false);
  const [entries, setEntries] = useState<McmEntryDto[]>([]);
  const [filter, setFilter] = useState("");
  const [compareDialogOpen, setCompareDialogOpen] = useState(false);
  const [comparePolicy, setComparePolicy] = useState<McmComparePolicy>("no_trans");
  const [compareResult, setCompareResult] = useState<McmCompareResult | null>(null);

  const handleOpen = async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "MCM Translation", extensions: ["txt"] },
        { name: "All", extensions: ["*"] },
      ],
    });
    if (!path) return;

    setLoading(true);
    try {
      const result = await loadMcmFile(path);
      setFile(result);
      setEntries(result.entries);
      setModified(false);
      setCompareResult(null);
      toast.success(t("mcm.loaded", { count: result.entry_count }));
    } catch (e: any) {
      toast.error(`${t("mcm.loadFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleEntryChange = (index: number, translation: string) => {
    const updated = [...entries];
    updated[index] = { ...updated[index], translation };
    setEntries(updated);
    setModified(true);
  };

  const handleSave = async () => {
    if (!file) return;
    try {
      await saveMcmFile({ path: file.path, entries });
      setModified(false);
      toast.success(t("mcm.saved"));
    } catch (e: any) {
      toast.error(`${t("mcm.saveFailed")}: ${e}`);
    }
  };

  const handleCompare = async () => {
    const refPath = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "MCM Translation", extensions: ["txt"] },
        { name: "All", extensions: ["*"] },
      ],
    });
    if (!refPath) return;

    try {
      const request: McmCompareRequest = {
        entries,
        reference_path: refPath as string,
        policy: comparePolicy,
      };
      const result = await mcmCompare(request);

      // Merge updated entries back into state
      if (result.updated_entries.length > 0) {
        const updatedMap = new Map(
          result.updated_entries.map((e) => [e.line_index, e])
        );
        setEntries((prev) =>
          prev.map((e) => updatedMap.get(e.line_index) || e)
        );
        setModified(true);
      }

      // Show result dialog instead of just toast
      setCompareResult(result);
      setCompareDialogOpen(false);

      toast.success(
        t("mcm.compareDone", {
          matched: result.matched,
          unmatched: result.unmatched,
          updated: result.updated_entries.length,
        })
      );
    } catch (e: any) {
      toast.error(`${t("mcm.compareFailed")}: ${e}`);
      setCompareDialogOpen(false);
    }
  };

  // ── Batch operations ──────────────────────────────────────────────

  const handleCopyAllSources = useCallback(() => {
    const updated = entries.map((e) => ({
      ...e,
      translation: e.translation || e.source,
    }));
    setEntries(updated);
    setModified(true);
    toast.success(t("mcm.copiedAllSources", { defaultValue: `Copied source to ${updated.filter(e => !e.translation && e.source).length} entries` }));
  }, [entries, t]);

  const handleClearAllTranslations = useCallback(() => {
    const updated = entries.map((e) => ({ ...e, translation: "" }));
    setEntries(updated);
    setModified(true);
    toast.success(t("mcm.clearedAll", { defaultValue: "All translations cleared" }));
  }, [entries, t]);

  const handleReverseSourceTranslation = useCallback(() => {
    const updated = entries.map((e) => ({
      ...e,
      source: e.translation || e.source,
      translation: e.source,
    }));
    setEntries(updated);
    setModified(true);
    toast.success(t("mcm.reversed", { defaultValue: "Source ↔ Translation reversed" }));
  }, [entries, t]);

  const hasAnyTranslation = entries.some((e) => e.translation.length > 0);
  const canCopySources = entries.some((e) => e.translation.length === 0 && e.source.length > 0);

  // ── Filtered entries ──────────────────────────────────────────────

  const filteredEntries = filter
    ? entries.filter(
        (e) =>
          e.source.toLowerCase().includes(filter.toLowerCase()) ||
          e.translation.toLowerCase().includes(filter.toLowerCase()) ||
          e.id.toLowerCase().includes(filter.toLowerCase())
      )
    : entries;

  const translatedCount = entries.filter((e) => getEntryStatus(e) === "translated").length;
  const partialCount = entries.filter((e) => getEntryStatus(e) === "partial").length;
  const untranslatedCount = entries.filter((e) => getEntryStatus(e) === "untranslated").length;

  // ── Stats breakdown ───────────────────────────────────────────────

  const statsBreakdown = useMemo(() => {
    const total = entries.length;
    const translated = translatedCount;
    const partial = partialCount;
    const untranslated = untranslatedCount;
    return { total, translated, partial, untranslated };
  }, [entries, translatedCount, partialCount, untranslatedCount]);

  return (
    <div className="sidepanel">
      {!file ? (
        <div className="sidepanel-empty">
          <EmptyState
            icon={<FileText size={36} />}
            title={t("mcm.title")}
            hint={t("mcm.subtitle")}
          />
          <Button variant="primary" onClick={handleOpen} disabled={loading} icon={<FileUp size={16} />} className="mcm-open-btn">
            {loading ? t("mcm.loading") : t("mcm.open")}
          </Button>
        </div>
      ) : (
        <>
          {/* ── File info ─────────────────────────────────────────── */}
          <div className="sidepanel-section">
            <h3>{t("mcm.fileInfo")}</h3>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("mcm.path")}</span>
              <span className="sidepanel-value" style={{ fontSize: 11, wordBreak: "break-all" }}>
                {file.path.split(/[/\\]/).pop()}
              </span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("mcm.encoding")}</span>
              <span className="sidepanel-value">{file.encoding}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("mcm.progress")}</span>
              <span className="sidepanel-value">
                {translatedCount} / {file.entry_count}
              </span>
            </div>

            {/* Progress bar */}
            <div className="mcm-progress-bar-track">
              <div
                className="mcm-progress-bar-fill"
                style={{ width: `${file.entry_count > 0 ? (translatedCount / file.entry_count) * 100 : 0}%` }}
              />
            </div>

            {/* Stats breakdown */}
            <div className="mcm-stats-breakdown">
              <span className="mcm-stat-chip mcm-stat-translated" title={t("mcm.translated", { defaultValue: "Translated" })}>
                ✓ {statsBreakdown.translated}
              </span>
              <span className="mcm-stat-chip mcm-stat-partial" title={t("mcm.partial", { defaultValue: "Partial" })}>
                ◐ {statsBreakdown.partial}
              </span>
              <span className="mcm-stat-chip mcm-stat-untranslated" title={t("mcm.untranslated", { defaultValue: "Untranslated" })}>
                ○ {statsBreakdown.untranslated}
              </span>
            </div>

            {/* Action buttons */}
            <div className="mcm-action-buttons">
              <Button variant="default" size="sm" onClick={handleOpen} icon={<FileUp size={12} />}>
                {t("mcm.openAnother")}
              </Button>
              <Button variant="primary" size="sm" onClick={handleSave} icon={<Save size={12} />} disabled={!modified}>
                {t("mcm.save")}
              </Button>
              <Button variant="default" size="sm" onClick={() => setCompareDialogOpen(true)} icon={<FileText size={12} />}>
                {t("mcm.compare")}
              </Button>
            </div>

            {/* Batch actions */}
            <div className="mcm-batch-actions">
              <Button variant="ghost" size="sm" onClick={handleCopyAllSources} disabled={!canCopySources} icon={<Copy size={11} />}>
                {t("mcm.copySources", { defaultValue: "Copy sources" })}
              </Button>
              <Button variant="ghost" size="sm" onClick={handleClearAllTranslations} disabled={!hasAnyTranslation} icon={<X size={11} />}>
                {t("mcm.clearAll", { defaultValue: "Clear all" })}
              </Button>
              <Button variant="ghost" size="sm" onClick={handleReverseSourceTranslation} disabled={!hasAnyTranslation} icon={<RotateCcw size={11} />}>
                {t("mcm.reverse", { defaultValue: "Reverse" })}
              </Button>
            </div>

            {modified && (
              <div style={{ marginTop: 6, width: "100%", textAlign: "center" }}>
                <Badge variant="incomplete">{t("mcm.unsaved")}</Badge>
              </div>
            )}

            {/* Compare policy dialog */}
            {compareDialogOpen && (
              <div className="mcm-compare-dialog">
                <div className="mcm-compare-label">
                  {t("mcm.comparePolicy")}
                </div>
                <Select
                  value={comparePolicy}
                  onChange={(e) => setComparePolicy(e.target.value as McmComparePolicy)}
                  size="sm"
                  className="mcm-compare-select"
                  options={[
                    { value: "all", label: t("mcm.policyAll") },
                    { value: "no_trans", label: t("mcm.policyNoTrans") },
                    { value: "no_trans_and_partial", label: t("mcm.policyNoTransAndPartial") },
                    { value: "partial_only", label: t("mcm.policyPartialOnly") },
                  ]}
                />
                <div className="mcm-compare-actions">
                  <Button variant="primary" size="sm" onClick={handleCompare}>
                    {t("mcm.compare")}
                  </Button>
                  <Button variant="default" size="sm" onClick={() => setCompareDialogOpen(false)}>
                    {t("common.cancel")}
                  </Button>
                </div>
              </div>
            )}
          </div>

          {/* ── Filter ────────────────────────────────────────────── */}
          <div className="sidepanel-section" style={{ padding: "8px" }}>
            <div className="mcm-filter-row">
              <input
                type="text"
                placeholder={t("mcm.filterPlaceholder")}
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                className="filter-input"
                style={{ flex: 1 }}
              />
              {filter && filteredEntries.length > 0 && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    const clearIndices = filteredEntries.map((e) =>
                      entries.findIndex((oe) => oe.line_index === e.line_index)
                    );
                    const updated = entries.map((e, i) =>
                      clearIndices.includes(i) ? { ...e, translation: "" } : e
                    );
                    setEntries(updated);
                    if (updated.some((e, i) => e.translation !== entries[i].translation)) {
                      setModified(true);
                    }
                  }}
                  icon={<RotateCcw size={12} />}
                >
                  {t("mcm.clearFiltered", { defaultValue: "Clear filtered" })}
                </Button>
              )}
            </div>
          </div>

          {/* ── Entry list ────────────────────────────────────────── */}
          <div className="sidepanel-section">
            <h3>
              {t("mcm.entriesCount", { count: filteredEntries.length })}
              {filter && ` (${t("mcm.filteredFrom", { total: entries.length })})`}
            </h3>
            <div style={{ maxHeight: 400, overflowY: "auto" }}>
              {filteredEntries.map((entry) => {
                const originalIndex = entries.findIndex((e) => e.line_index === entry.line_index);
                const status = getEntryStatus(entry);
                const statusCfg = STATUS_CONFIG[status];
                return (
                  <div
                    key={originalIndex}
                    className="record-type-row mcm-entry mcm-entry-compact"
                    data-status={status}
                  >
                    <div className="mcm-entry-header-row">
                      <span className="mcm-entry-id">{entry.id}</span>
                      <div className="mcm-entry-header-actions">
                        {/* Status badge */}
                        <span
                          className="mcm-entry-status-badge"
                          style={{ backgroundColor: statusCfg.color, color: "#fff" }}
                          title={t(statusCfg.labelKey)}
                        >
                          {statusCfg.icon}
                        </span>

                        {/* Char count ratio */}
                        {status !== "untranslated" && (
                          <span className="mcm-entry-char-count">
                            {entry.translation.length}/{entry.source.length}
                          </span>
                        )}

                        {/* Copy source button */}
                        <button
                          className="mcm-entry-copy-btn"
                          onClick={() => handleEntryChange(originalIndex, entry.source)}
                          title={t("mcm.copySource", { defaultValue: "Copy source as translation" })}
                        >
                          <Copy size={10} />
                        </button>
                      </div>
                    </div>
                    <div className="mcm-entry-body">
                      <div className="mcm-entry-source-cell">
                        {entry.source}
                      </div>
                      <textarea
                        value={entry.translation}
                        onChange={(e) => handleEntryChange(originalIndex, e.target.value)}
                        placeholder={t("mcm.translationPlaceholder")}
                        rows={2}
                        className="mcm-entry-textarea ui-textarea"
                      />
                    </div>
                  </div>
                );
              })}
              {filteredEntries.length === 0 && filter && (
                <div className="sidepanel-hint" style={{ padding: 16, textAlign: "center" }}>
                  {t("mcm.noMatch")}
                </div>
              )}
            </div>
          </div>
        </>
      )}

      {/* Compare result dialog */}
      {compareResult && (
        <CompareResultDialog
          result={compareResult}
          entries={entries}
          onClose={() => setCompareResult(null)}
        />
      )}
    </div>
  );
}
