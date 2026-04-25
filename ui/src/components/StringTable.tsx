import { useEffect, useCallback, ReactElement } from "react";
import { List } from "react-window";
import { useAppStore } from "../stores/appStore";
import { Search, ArrowUpDown, Loader2 } from "lucide-react";
import type { SkyStringDTO } from "../api/strings";

const ROW_HEIGHT = 32;

interface RowData {
  items: SkyStringDTO[];
  selectedId: number | null;
  onSelect: (id: number) => void;
}

function VirtualRow(props: {
  ariaAttributes: {
    "aria-posinset": number;
    "aria-setsize": number;
    role: "listitem";
  };
  index: number;
  style: React.CSSProperties;
  items: SkyStringDTO[];
  selectedId: number | null;
  onSelect: (id: number) => void;
}): ReactElement | null {
  const { index, style, items, selectedId, onSelect } = props;
  const item = items[index];
  if (!item) return null;

  const isSelected = selectedId === item.id;
  return (
    <div
      style={{
        ...style,
        display: "flex",
        alignItems: "center",
        borderBottom: "1px solid var(--border-subtle)",
        cursor: "pointer",
        background: isSelected
          ? "linear-gradient(90deg, rgba(0, 245, 255, 0.08), rgba(0, 245, 255, 0.02))"
          : "transparent",
        boxShadow: isSelected
          ? "inset 2px 0 0 var(--accent-cyan), 0 0 20px rgba(0, 245, 255, 0.1)"
          : "none",
        transition: "background 0.2s, box-shadow 0.2s",
      }}
      className={`status-${item.status}`}
      onClick={() => onSelect(item.id)}
      onMouseEnter={(e) => {
        if (!isSelected) {
          e.currentTarget.style.background = "var(--bg-secondary)";
        }
      }}
      onMouseLeave={(e) => {
        if (!isSelected) {
          e.currentTarget.style.background = "transparent";
        }
      }}
    >
      <div className="row-cell" style={{ width: 60, fontFamily: "'JetBrains Mono', monospace", fontSize: 11 }}>
        {item.id}
      </div>
      <div className="row-cell" style={{ width: 80 }}>
        <span className={`badge badge-${item.status}`}>
          {item.status[0].toUpperCase()}
        </span>
      </div>
      <div className="row-cell" style={{ width: 80, fontFamily: "'JetBrains Mono', monospace", fontSize: 11, color: "var(--accent-gold)" }}>
        {item.record_sig}
      </div>
      <div className="row-cell" style={{ width: 80, fontFamily: "'JetBrains Mono', monospace", fontSize: 11 }}>
        {item.field_sig}
      </div>
      <div className="row-cell form-id" style={{ width: 100 }}>
        {item.form_id}
      </div>
      <div
        className="row-cell text-cell source-text"
        style={{ flex: 1 }}
        title={item.source}
      >
        {item.source}
      </div>
      <div
        className="row-cell text-cell trans-text"
        style={{ flex: 1 }}
        title={item.translation}
      >
        {item.translation || "\u2014"}
      </div>
    </div>
  );
}

export function StringTable() {
  // Zustand selectors — stable references, no re-render on unrelated state changes
  const espPath = useAppStore((s) => s.espPath);
  const allItems = useAppStore((s) => s.allItems);
  const items = useAppStore((s) => s.items);
  const isLoading = useAppStore((s) => s.isLoading);
  const filter = useAppStore((s) => s.filter);
  const statusFilter = useAppStore((s) => s.statusFilter);
  const selectedId = useAppStore((s) => s.selectedId);
  const total = useAppStore((s) => s.total);
  const filtered = useAppStore((s) => s.filtered);

  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const setFilter = useAppStore((s) => s.setFilter);
  const setSort = useAppStore((s) => s.setSort);
  const setStatusFilter = useAppStore((s) => s.setStatusFilter);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const selectNextRow = useAppStore((s) => s.selectNextRow);
  const selectPrevRow = useAppStore((s) => s.selectPrevRow);

  // Keyboard navigation: arrow keys to move selection
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        selectNextRow();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        selectPrevRow();
      }
    },
    [selectNextRow, selectPrevRow]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // Load all strings when ESP is loaded
  useEffect(() => {
    if (espPath && allItems.length === 0) {
      loadAllStrings();
    }
  }, [espPath, allItems.length, loadAllStrings]);

  const handleSort = (field: string) => setSort(field);
  const handleSelect = (id: number) => setSelectedById(id);

  const rowData: RowData = {
    items,
    selectedId,
    onSelect: handleSelect,
  };

  if (isLoading && allItems.length === 0) {
    return (
      <div className="string-table-wrapper">
        <div className="table-loading">
          <Loader2 size={24} className="spin" />
          <span>Loading strings...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="string-table-wrapper">
      {/* Toolbar */}
      <div className="table-toolbar">
        <div className="search-box">
          <Search size={14} />
          <input
            type="text"
            placeholder="Filter strings..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="filter-input"
          />
        </div>
        <div className="status-filters">
          {[
            { key: null, label: "All" },
            { key: "incomplete", label: "Incomplete" },
            { key: "translated", label: "Translated" },
            { key: "locked", label: "Locked" },
          ].map((s) => (
            <button
              key={s.label}
              className={`status-filter-btn ${statusFilter === s.key ? "active" : ""}`}
              onClick={() =>
                setStatusFilter(
                  statusFilter === s.key ? null : (s.key as string)
                )
              }
            >
              {s.label}
            </button>
          ))}
        </div>
        <div className="table-info">
          {filtered.toLocaleString()} / {total.toLocaleString()}
          {allItems.length === 0 && !isLoading && (
            <span style={{ color: "var(--error)", marginLeft: 8 }}>(No data loaded)</span>
          )}
        </div>
      </div>

      {/* Header */}
      <div className="virtual-table-header">
        <div className="header-cell" style={{ width: 60 }} onClick={() => handleSort("id")}>
          ID <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ width: 80 }}>Status</div>
        <div className="header-cell" style={{ width: 80 }} onClick={() => handleSort("record_sig")}>
          Rec <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ width: 80 }}>Field</div>
        <div className="header-cell" style={{ width: 100 }}>FormID</div>
        <div className="header-cell" style={{ flex: 1 }}>Source</div>
        <div className="header-cell" style={{ flex: 1 }}>Translation</div>
      </div>

      {/* Virtual list */}
      <div className="virtual-list-container">
        <List<RowData>
          rowComponent={VirtualRow}
          rowCount={items.length}
          rowHeight={ROW_HEIGHT}
          rowProps={rowData}
          overscanCount={20}
          style={{ height: "100%", width: "100%" }}
        />
      </div>
    </div>
  );
}
