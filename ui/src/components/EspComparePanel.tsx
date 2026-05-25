import { useMemo, useState, useCallback, type CSSProperties, type ReactElement } from "react";
import { List } from "react-window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { GitCompare, FileUp, RefreshCw, Download, ArrowUpDown, ArrowUp, ArrowDown } from "lucide-react";
import toast from "react-hot-toast";
import { invoke } from "@tauri-apps/api/core";
import { compareEspFiles } from "../api/strings";
import type { EspCompareResultDto, EspComparePairDto } from "../api/strings";
import { Button, EmptyState, Input } from "./ui";

type Tab = "identical" | "added" | "removed" | "modified";

type SortField = "record_sig" | "field_sig";
type SortDir = "asc" | "desc" | null;

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

// ── 简单的文本差异对比 ──────────────────────────────────────────────
// 标记旧文本和新文本之间的不同部分。
// 返回用于渲染的 {text, type} 段列表。
interface DiffSegment {
  text: string;
  type: "same" | "del" | "add";
}

function computeDiff(oldText: string, newText: string): DiffSegment[] {
  if (oldText === newText) return [{ text: newText, type: "same" }];

  const minLen = Math.min(oldText.length, newText.length);

  // 寻找公共前缀
  let prefixEnd = 0;
  while (prefixEnd < minLen && oldText[prefixEnd] === newText[prefixEnd]) {
    prefixEnd++;
  }

  // 寻找公共后缀
  let suffixStart = 0;
  while (
    suffixStart < minLen - prefixEnd &&
    oldText[oldText.length - 1 - suffixStart] === newText[newText.length - 1 - suffixStart]
  ) {
    suffixStart++;
  }

  const segments: DiffSegment[] = [];

  // 前缀（相同）
  if (prefixEnd > 0) {
    segments.push({ text: oldText.slice(0, prefixEnd), type: "same" });
  }

  // 中间部分 — 在旧文本中被删除，在新文本中被添加
  const oldMid = oldText.slice(prefixEnd, oldText.length - suffixStart);
  const newMid = newText.slice(prefixEnd, newText.length - suffixStart);

  if (oldMid.length > 0) {
    segments.push({ text: oldMid, type: "del" });
  }
  if (newMid.length > 0) {
    segments.push({ text: newMid, type: "add" });
  }

  // 后缀（相同）
  if (suffixStart > 0) {
    segments.push({ text: oldText.slice(oldText.length - suffixStart), type: "same" });
  }

  return segments;
}

// ── 行组件 ───────────────────────────────────────────────────

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

  // 已修改条目的差异对比
  const diffSegments = activeTab === "modified"
    ? computeDiff(entry.old_source, entry.new_source)
    : null;

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

        {/* 旧的/当前的源文本 */}
        {activeTab !== "added" && (
          <div className="esp-compare-row-source" title={entry.old_source || entry.source}>
            <span className="esp-compare-row-label">OLD: </span>
            {entry.old_source || entry.source || <em className="esp-compare-empty">(empty)</em>}
          </div>
        )}

        {/* 新的/修改后的源文本或差异 */}
        {activeTab === "modified" && diffSegments ? (
          <div className="esp-compare-row-new" title={entry.new_source}>
            <span className="esp-compare-row-label-new">NEW: </span>
            {diffSegments.map((seg, i) => (
              <span
                key={i}
                className={
                  seg.type === "del"
                    ? "esp-diff-del"
                    : seg.type === "add"
                      ? "esp-diff-add"
                      : ""
                }
              >
                {seg.text}
              </span>
            ))}
          </div>
        ) : activeTab === "removed" ? null : (
          <div className="esp-compare-row-text" title={entry.source}>
            {entry.source || <em className="esp-compare-empty">(empty)</em>}
          </div>
        )}
      </div>
    </div>
  );
}

// ── 排序辅助函数 ──────────────────────────────────────────────────

function sortEntries(
  entries: EspComparePairDto[],
  field: SortField,
  dir: SortDir
): EspComparePairDto[] {
  if (!dir || !field) return entries;
  const sorted = [...entries].sort((a, b) => {
    const va = a[field].toLowerCase();
    const vb = b[field].toLowerCase();
    const cmp = va < vb ? -1 : va > vb ? 1 : 0;
    return dir === "asc" ? cmp : -cmp;
  });
  return sorted;
}

// ── 导出 ──────────────────────────────────────────────────────────

async function exportCompareReport(
  result: EspCompareResultDto,
  oldPath: string,
  newPath: string
): Promise<void> {
  const savePath = await save({
    defaultPath: `esp_compare_report_${Date.now()}.txt`,
    filters: [{ name: "Text", extensions: ["txt"] }],
  });
  if (!savePath) return;

  const reportLines: string[] = [];
  reportLines.push("=".repeat(60));
  reportLines.push("ESP Compare Report");
  reportLines.push("=".repeat(60));
  reportLines.push(`OLD: ${oldPath}`);
  reportLines.push(`NEW: ${newPath}`);
  reportLines.push("");
  reportLines.push(`Summary:`);
  reportLines.push(`  Identical: ${result.identical_count}`);
  reportLines.push(`  Added:     ${result.added_count}`);
  reportLines.push(`  Removed:   ${result.removed_count}`);
  reportLines.push(`  Modified:  ${result.modified_count}`);
  reportLines.push("");

  const section = (title: string, items: EspComparePairDto[], showNew: boolean) => {
    if (items.length === 0) return;
    reportLines.push(`--- ${title} (${items.length}) ---`);
    for (const item of items) {
      const oldIdStr = item.old_id ? `#${item.old_id.toString(16).toUpperCase()}` : "-";
      const newIdStr = item.new_id ? `#${item.new_id.toString(16).toUpperCase()}` : "-";
      reportLines.push(`  ${item.record_sig}/${item.field_sig} [old=${oldIdStr} new=${newIdStr}]`);
      reportLines.push(`    Old: ${item.old_source || item.source || "(empty)"}`);
      if (showNew) {
        reportLines.push(`    New: ${item.new_source || "(empty)"}`);
      }
      reportLines.push("");
    }
  };

  section("Identical", result.identical, false);
  section("Added", result.added, true);
  section("Removed", result.removed, false);
  section("Modified", result.modified, true);

  const reportContent = reportLines.join("\n");

  // 通过 Tauri invoke 写入 — 调用 Rust 的 write_text_file 命令
  try {
    await invoke("write_text_file", { path: savePath, content: reportContent });
    toast.success(`Report saved to ${savePath.split(/[/\\]/).pop()}`);
  } catch (e: any) {
    toast.error(`Export failed: ${e}`);
  }
}

// ── 主组件 ──────────────────────────────────────────────────

export function EspComparePanel() {
  const { t } = useTranslation();
  const [compareResult, setCompareResult] = useState<CompareResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("identical");
  const [filterText, setFilterText] = useState("");
  const [sortField, setSortField] = useState<SortField | null>(null);
  const [sortDir, setSortDir] = useState<SortDir>(null);
  const [exporting, setExporting] = useState(false);

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
      setSortField(null);
      setSortDir(null);
      setFilterText("");
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

  const handleExport = useCallback(async () => {
    if (!compareResult) return;
    setExporting(true);
    try {
      await exportCompareReport(compareResult.result, compareResult.oldPath, compareResult.newPath);
    } catch (e: any) {
      toast.error(`Export failed: ${e}`);
    } finally {
      setExporting(false);
    }
  }, [compareResult]);

  const handleSortToggle = useCallback((field: SortField) => {
    setSortField((prev) => {
      if (prev !== field) return field;
      return field;
    });
    setSortDir((prev) => {
      if (prev === null) return "asc";
      if (prev === "asc") return "desc";
      return null;
    });
  }, []);

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
    let filtered = !q
      ? tabEntries
      : tabEntries.filter(
          (e) =>
            e.source.toLowerCase().includes(q) ||
            e.old_source.toLowerCase().includes(q) ||
            e.new_source.toLowerCase().includes(q) ||
            e.record_sig.toLowerCase().includes(q) ||
            e.field_sig.toLowerCase().includes(q)
        );
    if (sortField && sortDir) {
      filtered = sortEntries(filtered, sortField, sortDir);
    }
    return filtered;
  }, [activeTab, compareResult, filterText, sortField, sortDir]);

  const SortIcon = sortDir === "asc" ? ArrowUp : sortDir === "desc" ? ArrowDown : ArrowUpDown;

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

  const totalEntries = compareResult
    ? compareResult.result.identical_count + compareResult.result.added_count + compareResult.result.removed_count + compareResult.result.modified_count
    : 0;

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
          {/* 头部 */}
          <div className="sidepanel-section esp-compare-header">
            <div className="esp-compare-header-row">
              <h3 style={{ margin: 0 }}>{t("espCompare.title")}</h3>
              <div className="esp-compare-header-actions">
                <Button variant="ghost" size="sm" onClick={handleExport} disabled={exporting} title={t("espCompare.export")} icon={<Download size={14} />}>
                  {t("espCompare.export")}
                </Button>
                <Button variant="ghost" size="sm" onClick={handleCompare} disabled={loading} title={t("espCompare.compareAgain")} icon={<RefreshCw size={14} />} />
              </div>
            </div>
            <div className="esp-compare-paths">
              <div>OLD: {compareResult.oldPath.replace(/\\/g, "/").split("/").pop()}</div>
              <div>NEW: {compareResult.newPath.replace(/\\/g, "/").split("/").pop()}</div>
            </div>
          </div>

          {/* 摘要栏 */}
          <div className="esp-compare-summary-bar">
            <span className="esp-compare-summary-identical">{t("espCompare.tabs.identical")}: {tabCounts.identical.toLocaleString()}</span>
            <span className="esp-compare-summary-added">+{tabCounts.added.toLocaleString()}</span>
            <span className="esp-compare-summary-removed">-{tabCounts.removed.toLocaleString()}</span>
            <span className="esp-compare-summary-modified">~{tabCounts.modified.toLocaleString()}</span>
            <span className="esp-compare-summary-total">/ {totalEntries.toLocaleString()}</span>
          </div>

          {/* 标签栏 */}
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

          {/* 排序 + 过滤栏 */}
          <div className="esp-compare-filter">
            <div className="esp-compare-sort-row">
              <button
                className={`esp-compare-sort-btn ${sortField === "record_sig" ? "esp-compare-sort-active" : ""}`}
                onClick={() => handleSortToggle("record_sig")}
              >
                <SortIcon size={10} />
                {t("espCompare.recordSig", { defaultValue: "Record" })}
              </button>
              <button
                className={`esp-compare-sort-btn ${sortField === "field_sig" ? "esp-compare-sort-active" : ""}`}
                onClick={() => handleSortToggle("field_sig")}
              >
                <SortIcon size={10} />
                {t("espCompare.fieldSig", { defaultValue: "Field" })}
              </button>
            </div>
            <Input
              size="sm"
              placeholder={t("espCompare.filterPlaceholder")}
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
            />
          </div>

          {/* 结果计数 */}
          <div className="esp-compare-result-count">
            {t("espCompare.showing", { count: entries.length, total: tabCounts[activeTab].toLocaleString() })}
          </div>

          {/* 条目列表 */}
          <div style={{ height: "calc(100vh - 290px)", minHeight: 200 }}>
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
