import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { FileText, FileUp, Save } from "lucide-react";
import toast from "react-hot-toast";
import { loadMcmFile, saveMcmFile, mcmCompare } from "../api/strings";
import type { McmFileDto, McmEntryDto, McmComparePolicy, McmCompareRequest } from "../api/strings";

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
          <FileText size={36} />
          <p style={{ marginTop: 8 }}>{t("mcm.title")}</p>
          <p className="sidepanel-hint">{t("mcm.subtitle")}</p>
          <button
            onClick={handleOpen}
            disabled={loading}
            className="btn btn-primary"
            style={{ marginTop: 16 }}
          >
            <FileUp size={16} />
            <span>{loading ? t("mcm.loading") : t("mcm.open")}</span>
          </button>
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
            <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
              <button onClick={handleOpen} className="btn btn-sm" style={{ flex: 1 }}>
                <FileUp size={12} /> {t("mcm.openAnother")}
              </button>
              <button
                onClick={handleSave}
                className="btn btn-sm btn-primary"
                style={{ flex: 1 }}
                disabled={!modified}
              >
                <Save size={12} /> {t("mcm.save")}
              </button>
              <button
                onClick={() => setCompareDialogOpen(true)}
                className="btn btn-sm"
                style={{ flex: 1 }}
              >
                <FileText size={12} /> {t("mcm.compare")}
              </button>
            </div>
            {modified && (
              <div className="badge badge-warning" style={{ marginTop: 6, width: "100%", textAlign: "center" }}>
                {t("mcm.unsaved")}
              </div>
            )}

            {/* Compare dialog */}
            {compareDialogOpen && (
              <div
                style={{
                  marginTop: 8,
                  padding: 8,
                  border: "1px solid var(--border-color)",
                  borderRadius: 4,
                  background: "var(--bg-elevated)",
                }}
              >
                <div style={{ marginBottom: 8, fontSize: 12 }}>
                  {t("mcm.comparePolicy")}
                </div>
                <select
                  value={comparePolicy}
                  onChange={(e) => setComparePolicy(e.target.value as McmComparePolicy)}
                  className="policy-select"
                  style={{
                    width: "100%",
                    padding: "4px 8px",
                    borderRadius: 4,
                    border: "1px solid var(--border-color)",
                    background: "var(--bg-base)",
                    color: "var(--text-primary)",
                    marginBottom: 8,
                  }}
                >
                  <option value="all">{t("mcm.policyAll")}</option>
                  <option value="no_trans">{t("mcm.policyNoTrans")}</option>
                  <option value="no_trans_and_partial">{t("mcm.policyNoTransAndPartial")}</option>
                  <option value="partial_only">{t("mcm.policyPartialOnly")}</option>
                </select>
                <div style={{ display: "flex", gap: 4 }}>
                  <button
                    onClick={handleCompare}
                    className="btn btn-sm btn-primary"
                    style={{ flex: 1 }}
                  >
                    {t("mcm.compare")}
                  </button>
                  <button
                    onClick={() => setCompareDialogOpen(false)}
                    className="btn btn-sm"
                    style={{ flex: 1 }}
                  >
                    {t("common.cancel")}
                  </button>
                </div>
              </div>
            )}
          </div>

          {/* Filter */}
          <div className="sidepanel-section" style={{ padding: "8px" }}>
            <input
              type="text"
              placeholder={t("mcm.filterPlaceholder")}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="filter-input"
              style={{ width: "100%" }}
            />
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
                    className="record-type-row"
                    style={{ padding: "8px", lineHeight: 1.4 }}
                  >
                    <div style={{ fontSize: 10, color: "var(--text-muted)", marginBottom: 4 }}>
                      {entry.id}
                    </div>
                    <div
                      style={{
                        fontSize: 11,
                        color: "var(--text-primary)",
                        marginBottom: 6,
                        padding: "4px 6px",
                        background: "var(--bg-elevated)",
                        borderRadius: 4,
                        fontFamily: "monospace",
                      }}
                    >
                      {entry.source}
                    </div>
                    <textarea
                      value={entry.translation}
                      onChange={(e) => handleEntryChange(originalIndex, e.target.value)}
                      placeholder={t("mcm.translationPlaceholder")}
                      rows={2}
                      className="translation-textarea"
                      style={{
                        width: "100%",
                        resize: "vertical",
                        fontSize: 11,
                        padding: "4px 6px",
                        borderRadius: 4,
                        border: "1px solid var(--border-color)",
                        background: "var(--bg-elevated)",
                        color: entry.translation ? "var(--text-primary)" : "var(--text-muted)",
                      }}
                    />
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