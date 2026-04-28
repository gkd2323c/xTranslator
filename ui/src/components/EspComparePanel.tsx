import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { GitCompare, FileUp, RefreshCw } from "lucide-react";
import toast from "react-hot-toast";
import { compareEspFiles } from "../api/strings";
import type { EspCompareResultDto, EspComparePairDto } from "../api/strings";

type Tab = "identical" | "added" | "removed" | "modified";

interface CompareResult {
  result: EspCompareResultDto;
  oldPath: string;
  newPath: string;
}

export function EspComparePanel() {
  const { t } = useTranslation();
  const [compareResult, setCompareResult] = useState<CompareResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("identical");
  const [filterText, setFilterText] = useState("");

  const handleCompare = async () => {
    const oldFile = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "ESP/ESM", extensions: ["esp", "esm"] }],
      title: "Select OLD (original) ESP/ESM",
    });
    if (!oldFile) return;

    const newFile = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "ESP/ESM", extensions: ["esp", "esm"] }],
      title: "Select NEW (updated) ESP/ESM",
    });
    if (!newFile) return;

    setLoading(true);
    try {
      const result = await compareEspFiles(oldFile, newFile);
      setCompareResult({ result, oldPath: oldFile, newPath: newFile });
      setActiveTab("identical");
      toast.success(
        `${t("espCompare.loaded", { total: result.identical_count + result.added_count + result.removed_count + result.modified_count })} ` +
        `[${result.identical_count} identical, +${result.added_count} added, -${result.removed_count} removed, ~${result.modified_count} modified]`
      );
    } catch (e: any) {
      toast.error(`${t("espCompare.loadFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const getEntries = (): EspComparePairDto[] => {
    if (!compareResult) return [];
    const tabMap: Record<Tab, EspComparePairDto[]> = {
      identical: compareResult.result.identical,
      added: compareResult.result.added,
      removed: compareResult.result.removed,
      modified: compareResult.result.modified,
    };
    let entries = tabMap[activeTab] ?? [];
    if (filterText.trim()) {
      const q = filterText.toLowerCase();
      entries = entries.filter(
        (e) =>
          e.source.toLowerCase().includes(q) ||
          e.old_source.toLowerCase().includes(q) ||
          e.new_source.toLowerCase().includes(q) ||
          e.record_sig.toLowerCase().includes(q) ||
          e.field_sig.toLowerCase().includes(q)
      );
    }
    return entries;
  };

  const tabCounts: Record<Tab, number> = compareResult
    ? {
        identical: compareResult.result.identical_count,
        added: compareResult.result.added_count,
        removed: compareResult.result.removed_count,
        modified: compareResult.result.modified_count,
      }
    : { identical: 0, added: 0, removed: 0, modified: 0 };

  return (
    <div className="sidepanel">
      {!compareResult ? (
        <div className="sidepanel-empty">
          <GitCompare size={36} />
          <p style={{ marginTop: 8 }}>{t("espCompare.title")}</p>
          <p className="sidepanel-hint">{t("espCompare.subtitle")}</p>
          <button onClick={handleCompare} disabled={loading} className="btn btn-primary" style={{ marginTop: 16 }}>
            <FileUp size={16} />
            <span>{loading ? t("espCompare.comparing") : t("espCompare.compare")}</span>
          </button>
        </div>
      ) : (
        <>
          {/* Header */}
          <div className="sidepanel-section" style={{ paddingBottom: 0 }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
              <h3 style={{ margin: 0 }}>{t("espCompare.title")}</h3>
              <button onClick={handleCompare} disabled={loading} className="btn btn-ghost btn-sm" title={t("espCompare.compareAgain")}>
                <RefreshCw size={14} />
              </button>
            </div>
            <div style={{ fontSize: 10, color: "var(--text-muted)", lineHeight: 1.4 }}>
              <div>OLD: {compareResult.oldPath.replace(/\\/g, "/").split("/").pop()}</div>
              <div>NEW: {compareResult.newPath.replace(/\\/g, "/").split("/").pop()}</div>
            </div>
          </div>

          {/* Tab bar */}
          <div style={{ display: "flex", borderBottom: "1px solid var(--border)", margin: "8px 12px 0" }}>
            {(["identical", "added", "removed", "modified"] as Tab[]).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                style={{
                  flex: 1,
                  padding: "6px 4px",
                  background: "none",
                  border: "none",
                  borderBottom: activeTab === tab ? "2px solid var(--accent)" : "2px solid transparent",
                  color: activeTab === tab ? "var(--accent)" : "var(--text-secondary)",
                  fontSize: 11,
                  cursor: "pointer",
                }}
              >
                {t(`espCompare.tabs.${tab}`)} ({tabCounts[tab].toLocaleString()})
              </button>
            ))}
          </div>

          {/* Filter */}
          <div style={{ padding: "8px 12px" }}>
            <input
              type="text"
              placeholder={t("espCompare.filterPlaceholder")}
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
              style={{
                width: "100%",
                padding: "6px 8px",
                border: "1px solid var(--border)",
                borderRadius: 4,
                background: "var(--bg-secondary)",
                color: "var(--text-primary)",
                fontSize: 12,
                boxSizing: "border-box",
              }}
            />
          </div>

          {/* Entry list */}
          <div style={{ flex: 1, overflowY: "auto", padding: "0 12px" }}>
            {getEntries().length === 0 ? (
              <div style={{ textAlign: "center", color: "var(--text-muted)", padding: 20, fontSize: 12 }}>
                {t("espCompare.noMatch")}
              </div>
            ) : (
              getEntries().map((entry, idx) => (
                <div key={idx} className="record-type-row" style={{ padding: "8px", marginBottom: 4, background: "var(--bg-secondary)", borderRadius: 4 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                    <span style={{ fontSize: 10, fontFamily: "monospace", color: "var(--text-muted)" }}>
                      {entry.record_sig}/{entry.field_sig} [old=#{entry.old_id.toString(16).toUpperCase()} new=#{entry.new_id.toString(16).toUpperCase()}]
                    </span>
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 2 }}>
                    <span style={{ color: "var(--text-muted)" }}>OLD: </span>
                    {entry.old_source ? entry.old_source : <em style={{ opacity: 0.5 }}>(empty)</em>}
                  </div>
                  {activeTab === "modified" && (
                    <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                      <span style={{ color: "var(--accent)" }}>NEW: </span>
                      {entry.new_source ? entry.new_source : <em style={{ opacity: 0.5 }}>(empty)</em>}
                    </div>
                  )}
                  {activeTab !== "modified" && (
                    <div style={{ fontSize: 11, color: "var(--text-primary)", marginTop: 2 }}>
                      {entry.source ? entry.source : <em style={{ opacity: 0.5 }}>(empty)</em>}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </>
      )}
    </div>
  );
}