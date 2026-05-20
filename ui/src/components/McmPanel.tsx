import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { FileText, FileUp, Save, RotateCcw, CheckCircle2, Copy } from "lucide-react";
import toast from "react-hot-toast";
import { loadMcmFile, saveMcmFile, mcmCompare } from "../api/strings";
import type { McmFileDto, McmEntryDto, McmComparePolicy, McmCompareRequest } from "../api/strings";
import { Button, Badge, EmptyState, Select } from "./ui";

export function McmPanel() {
  const { t } = useTranslation();
  const [file, setFile] = useState<McmFileDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [modified, setModified] = useState(false);
  const [entries, setEntries] = useState<McmEntryDto[]>([]);
  const [filter, setFilter] = useState("");
  const [compareDialogOpen, setCompareDialogOpen] = useState(false);
  const [comparePolicy, setComparePolicy] = useState<McmComparePolicy>("no_trans");

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

      toast.success(
        t("mcm.compareDone", {
          matched: result.matched,
          unmatched: result.unmatched,
          updated: result.updated_entries.length,
        })
      );
    } catch (e: any) {
      toast.error(`${t("mcm.compareFailed")}: ${e}`);
    }
    setCompareDialogOpen(false);
  };

  const filteredEntries = filter
    ? entries.filter(
        (e) =>
          e.source.toLowerCase().includes(filter.toLowerCase()) ||
          e.translation.toLowerCase().includes(filter.toLowerCase()) ||
          e.id.toLowerCase().includes(filter.toLowerCase())
      )
    : entries;

  const translatedCount = entries.filter((e) => e.translation.length > 0).length;

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
          {/* File info */}
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
            {/* 进度条 */}
            <div className="mcm-progress-bar-track">
              <div
                className="mcm-progress-bar-fill"
                style={{ width: `${file.entry_count > 0 ? (translatedCount / file.entry_count) * 100 : 0}%` }}
              />
            </div>
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
            {modified && (
              <div style={{ marginTop: 6, width: "100%", textAlign: "center" }}>
                <Badge variant="incomplete">{t("mcm.unsaved")}</Badge>
              </div>
            )}

            {/* Compare dialog */}
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

          {/* Filter */}
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

          {/* Entry list */}
          <div className="sidepanel-section">
            <h3>
              {t("mcm.entriesCount", { count: filteredEntries.length })}
              {filter && ` (${t("mcm.filteredFrom", { total: entries.length })})`}
            </h3>
            <div style={{ maxHeight: 400, overflowY: "auto" }}>
              {filteredEntries.map((entry) => {
                const originalIndex = entries.findIndex((e) => e.line_index === entry.line_index);
                return (
                  <div
                    key={originalIndex}
                    className="record-type-row mcm-entry mcm-entry-compact"
                  >
                    <div className="mcm-entry-header-row">
                      <span className="mcm-entry-id">{entry.id}</span>
                      <div className="mcm-entry-header-actions">
                        {entry.translation.length > 0 && (
                          <>
                            <CheckCircle2 size={11} className="mcm-entry-translated-icon" />
                            <span className="mcm-entry-char-count">
                              {entry.translation.length}/{entry.source.length}
                            </span>
                          </>
                        )}
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
    </div>
  );
}