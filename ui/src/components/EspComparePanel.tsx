import { useMemo, useState, type CSSProperties, type ReactElement } from "react";
import { List } from "react-window";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { GitCompare, FileUp, RefreshCw } from "lucide-react";
import toast from "react-hot-toast";
import { compareEspFiles } from "../api/strings";
import type { EspCompareResultDto, EspComparePairDto } from "../api/strings";
import { Button, EmptyState, Input } from "./ui";

type Tab = "identical" | "added" | "removed" | "modified";

interface CompareResult {
  result: EspCompareResultDto;
  oldPath: string;
  newPath: string;
}

interface CompareRowData {
  entries: EspComparePairDto[];
  activeTab: Tab;
}

const ROW_HEIGHT = 94;

function CompareRow(props: {
  ariaAttributes: {
    "aria-posinset": number;
    "aria-setsize": number;
    role: "listitem";
  };
  index: number;
  style: CSSProperties;
  entries: EspComparePairDto[];
  activeTab: Tab;
}): ReactElement | null {
  const { ariaAttributes, index, style, entries, activeTab } = props;
  const entry = entries[index];
  if (!entry) return null;

  const oldId = activeTab === "added" ? "-" : `#${entry.old_id.toString(16).toUpperCase()}`;
  const newId = activeTab === "removed" ? "-" : `#${entry.new_id.toString(16).toUpperCase()}`;

  return (
    <div
      {...ariaAttributes}
      style={{
        ...style,
        boxSizing: "border-box",
        padding: "2px 12px",
      }}
    >
      <div className="esp-compare-row">
        <div className="esp-compare-row-header">
          <span
            className="esp-compare-row-sig"
            title={`${entry.record_sig}/${entry.field_sig} [old=${oldId} new=${newId}]`}
          >
            {entry.record_sig}/{entry.field_sig} [old={oldId} new={newId}]
          </span>
        </div>
        <div className="esp-compare-row-source" title={entry.old_source || entry.source}>
          <span className="esp-compare-row-label">OLD: </span>
          {entry.old_source || entry.source || <em className="esp-compare-empty">(empty)</em>}
        </div>
        {activeTab === "modified" ? (
          <div className="esp-compare-row-new" title={entry.new_source}>
            <span className="esp-compare-row-label-new">NEW: </span>
            {entry.new_source || <em className="esp-compare-empty">(empty)</em>}
          </div>
        ) : (
          <div className="esp-compare-row-text" title={entry.source}>
            {entry.source || <em className="esp-compare-empty">(empty)</em>}
          </div>
        )}
      </div>
    </div>
  );
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

  const entries = useMemo(() => {
    if (!compareResult) return [];
    const tabMap: Record<Tab, EspComparePairDto[]> = {
      identical: compareResult.result.identical,
      added: compareResult.result.added,
      removed: compareResult.result.removed,
      modified: compareResult.result.modified,
    };
    const tabEntries = tabMap[activeTab] ?? [];
    const q = filterText.trim().toLowerCase();
    if (!q) return tabEntries;
    return tabEntries.filter(
      (e) =>
        e.source.toLowerCase().includes(q) ||
        e.old_source.toLowerCase().includes(q) ||
        e.new_source.toLowerCase().includes(q) ||
        e.record_sig.toLowerCase().includes(q) ||
        e.field_sig.toLowerCase().includes(q)
    );
  }, [activeTab, compareResult, filterText]);

  const rowData: CompareRowData = {
    entries,
    activeTab,
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
          <EmptyState
            icon={<GitCompare size={36} />}
            title={t("espCompare.title")}
            hint={t("espCompare.subtitle")}
          />
          <Button variant="primary" onClick={handleCompare} disabled={loading} icon={<FileUp size={16} />} className="esp-compare-open-btn">
            {loading ? t("espCompare.comparing") : t("espCompare.compare")}
          </Button>
        </div>
      ) : (
        <>
          {/* Header */}
          <div className="sidepanel-section esp-compare-header">
            <div className="esp-compare-header-row">
              <h3 style={{ margin: 0 }}>{t("espCompare.title")}</h3>
              <Button variant="ghost" size="sm" onClick={handleCompare} disabled={loading} title={t("espCompare.compareAgain")} icon={<RefreshCw size={14} />} />
            </div>
            <div className="esp-compare-paths">
              <div>OLD: {compareResult.oldPath.replace(/\\/g, "/").split("/").pop()}</div>
              <div>NEW: {compareResult.newPath.replace(/\\/g, "/").split("/").pop()}</div>
            </div>
          </div>

          {/* Tab bar */}
          <div className="esp-compare-tabs">
            {(["identical", "added", "removed", "modified"] as Tab[]).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`esp-compare-tab ${activeTab === tab ? "esp-compare-tab-active" : ""}`}
              >
                {t(`espCompare.tabs.${tab}`)} ({tabCounts[tab].toLocaleString()})
              </button>
            ))}
          </div>

          {/* Filter */}
          <div className="esp-compare-filter">
            <Input
              size="sm"
              placeholder={t("espCompare.filterPlaceholder")}
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
            />
          </div>

          {/* Entry list */}
          <div style={{ height: "calc(100vh - 245px)", minHeight: 240 }}>
            {entries.length === 0 ? (
              <div className="esp-compare-empty">
                {t("espCompare.noMatch")}
              </div>
            ) : (
              <List<CompareRowData>
                rowComponent={CompareRow}
                rowCount={entries.length}
                rowHeight={ROW_HEIGHT}
                rowProps={rowData}
                overscanCount={8}
                style={{ height: "100%", width: "100%" }}
              />
            )}
          </div>
        </>
      )}
    </div>
  );
}
