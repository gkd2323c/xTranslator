import { useState, useEffect, useCallback, ReactElement } from "react";
import { List } from "react-window";
import { useAppStore } from "../stores/appStore";
import { ArrowUpDown, Replace, Edit3, Copy, Filter } from "lucide-react";
import type { SkyStringDTO } from "../api/strings";
import { useTranslation } from "react-i18next";
import { Input, Button, Spinner } from "./ui";
import { ContextMenu } from "./ContextMenu";

const ROW_HEIGHT = 32;

interface RowData {
  items: SkyStringDTO[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  onDoubleClick: (id: number) => void;
  onContextMenu: (e: React.MouseEvent, item: SkyStringDTO) => void;
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
  onDoubleClick: (id: number) => void;
  onContextMenu: (e: React.MouseEvent, item: SkyStringDTO) => void;
}): ReactElement | null {
  const { index, style, items, selectedId, onSelect, onDoubleClick, onContextMenu } = props;
  const item = items[index];
  if (!item) return null;

  const isSelected = selectedId === item.id;
  return (
    <div
      style={style}
      className={`virtual-row status-${item.status} ${isSelected ? "virtual-row-selected" : ""}`}
      onClick={() => onSelect(item.id)}
      onDoubleClick={() => onDoubleClick(item.id)}
      onContextMenu={(e) => onContextMenu(e, item)}
      onMouseEnter={(e) => {
        if (!isSelected) {
          e.currentTarget.classList.add("virtual-row-hover");
        }
      }}
      onMouseLeave={(e) => {
        if (!isSelected) {
          e.currentTarget.classList.remove("virtual-row-hover");
        }
      }}
    >
      <div className="row-cell row-cell-status-icon" title={`${item.record_sig}:${item.field_sig} #${item.form_id}`}>
        <span className={`status-dot status-${item.status}${item.is_vmad ? " status-vmad" : ""}`}>
          {item.status === "translated" ? "●" : item.status === "locked" ? "◆" : "○"}
        </span>
      </div>
      <div className="row-cell row-cell-edid" title={`${item.record_sig}:${item.field_sig}`}>
        {item.record_sig}:{item.field_sig}
      </div>
      <div className="row-cell row-cell-id">{item.id}</div>
      <div className="row-cell text-cell source-text" title={item.source}>
        {item.source}
      </div>
      <div className="row-cell text-cell trans-text" title={item.translation}>
        {item.translation || "—"}
      </div>
      <div className="row-cell row-cell-ld">
        {(item as any).ld ?? "—"}
      </div>
    </div>
  );
}

export function StringTable() {
  const { t } = useTranslation();
  const espPath = useAppStore((s) => s.espPath);
  const allItems = useAppStore((s) => s.allItems);
  const items = useAppStore((s) => s.items);
  const isLoading = useAppStore((s) => s.isLoading);
  const filter = useAppStore((s) => s.filter);
  const replaceText = useAppStore((s) => s.replaceText);
  const listIndex = useAppStore((s) => s.listIndex);
  const selectedId = useAppStore((s) => s.selectedId);
  const total = useAppStore((s) => s.total);
  const filtered = useAppStore((s) => s.filtered);

  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const setFilter = useAppStore((s) => s.setFilter);
  const setReplaceText = useAppStore((s) => s.setReplaceText);
  const setSort = useAppStore((s) => s.setSort);
  const setListIndex = useAppStore((s) => s.setListIndex);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const selectNextRow = useAppStore((s) => s.selectNextRow);
  const selectPrevRow = useAppStore((s) => s.selectPrevRow);
  const replaceAll = useAppStore((s) => s.replaceAll);
  const openEditorForItem = useAppStore((s) => s.openEditorForItem);

  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; item: SkyStringDTO } | null>(null);

  const handleContextMenu = useCallback((e: React.MouseEvent, item: SkyStringDTO) => {
    e.preventDefault();
    setSelectedById(item.id);
    setCtxMenu({ x: e.clientX, y: e.clientY, item });
  }, [setSelectedById]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        selectNextRow();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        selectPrevRow();
      } else if (e.key === "Enter" && selectedId !== null) {
        e.preventDefault();
        openEditorForItem(selectedId);
      }
    },
    [selectNextRow, selectPrevRow, selectedId, openEditorForItem]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

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
    onDoubleClick: (id) => openEditorForItem(id),
    onContextMenu: handleContextMenu,
  };

  if (isLoading && allItems.length === 0) {
    return (
      <div className="string-table-wrapper">
        <div className="table-loading">
          <Spinner size={24} />
          <span>{t("table.loading")}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="string-table-wrapper">
      <div className="list-index-tabs">
        {[
          { key: null, label: t("common.all", { defaultValue: "All" }) },
          { key: 0, label: "STRINGS" },
          { key: 1, label: "DLSTRINGS" },
          { key: 2, label: "ILSTRINGS" },
        ].map((tab) => (
          <button
            key={tab.label}
            className={`list-index-tab ${listIndex === tab.key ? "list-index-tab-active" : ""}`}
            onClick={() => setListIndex(tab.key)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="table-toolbar">
        <div className="table-toolbar-row">
          <div className="table-info">
            {filtered.toLocaleString()} / {total.toLocaleString()}
            {allItems.length === 0 && !isLoading && (
              <span className="table-info-error">{t("common.noDataLoaded")}</span>
            )}
          </div>
        </div>
        {filter && (
          <div className="table-toolbar-row">
            <Replace size={14} className="replace-icon" />
            <Input
              size="sm"
              placeholder="Replacement text (use $1, $2 for groups)..."
              value={replaceText}
              onChange={(e) => setReplaceText(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter" && replaceText) replaceAll(); }}
              wrapperClassName="replace-input-wrap"
            />
            <Button
              variant="default"
              size="sm"
              onClick={replaceAll}
              disabled={!replaceText}
            >
              Replace All
            </Button>
          </div>
        )}
      </div>

      <div className="virtual-table-header">
        <div className="header-cell" style={{ width: 28 }} />
        <div className="header-cell" style={{ width: 100 }} onClick={() => handleSort("record_sig")}>
          EDID <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ width: 60 }} onClick={() => handleSort("id")}>
          ID <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ flex: 1 }}>{t("table.source")}</div>
        <div className="header-cell" style={{ flex: 1 }}>{t("table.translation")}</div>
        <div className="header-cell" style={{ width: 40 }}>LD</div>
      </div>

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
      {ctxMenu && (
        <ContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          onClose={() => setCtxMenu(null)}
          items={[
            {
              label: t("table.ctxEdit", { defaultValue: "Edit" }),
              icon: <Edit3 size={14} />,
              shortcut: "Enter",
              onClick: () => setSelectedById(ctxMenu.item.id),
            },
            { separator: true, label: "" },
            {
              label: t("table.ctxCopySource", { defaultValue: "Copy Source" }),
              icon: <Copy size={14} />,
              shortcut: "Ctrl+C",
              onClick: () => navigator.clipboard.writeText(ctxMenu.item.source),
            },
            {
              label: t("table.ctxCopyTranslation", { defaultValue: "Copy Translation" }),
              icon: <Copy size={14} />,
              onClick: () => navigator.clipboard.writeText(ctxMenu.item.translation || ""),
              disabled: !ctxMenu.item.translation,
            },
            { separator: true, label: "" },
            {
              label: t("table.ctxFilterFormId", { defaultValue: "Filter by FormID" }),
              icon: <Filter size={14} />,
              shortcut: "F12",
              onClick: () => setFilter(ctxMenu.item.form_id),
            },
          ]}
        />
      )}
    </div>
  );
}
