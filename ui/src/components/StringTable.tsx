import { useEffect, useCallback, ReactElement } from "react";
import { List } from "react-window";
import { useAppStore } from "../stores/appStore";
import { Search, ArrowUpDown, Code2, Replace } from "lucide-react";
import type { SkyStringDTO } from "../api/strings";
import { useTranslation } from "react-i18next";
import { Input, Button, Badge, Spinner } from "./ui";

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
  const { t } = useTranslation();
  const { index, style, items, selectedId, onSelect } = props;
  const item = items[index];
  if (!item) return null;

  const isSelected = selectedId === item.id;
  return (
    <div
      style={style}
      className={`virtual-row status-${item.status} ${isSelected ? "virtual-row-selected" : ""}`}
      onClick={() => onSelect(item.id)}
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
      <div className="row-cell row-cell-id">{item.id}</div>
      <div className="row-cell row-cell-status">
        <Badge variant={item.status === "translated" ? "translated" : item.status === "incomplete" ? "incomplete" : "locked"} size="sm">
          {item.status[0].toUpperCase()}
        </Badge>
        {item.is_vmad && <span title={t("table.vmadTooltip")}><Badge variant="script" size="sm">VM</Badge></span>}
      </div>
      <div className="row-cell row-cell-rec">{item.record_sig}</div>
      <div className="row-cell row-cell-field">{item.field_sig}</div>
      <div className="row-cell row-cell-formid">{item.form_id}</div>
      <div className="row-cell text-cell source-text" title={item.source}>
        {item.source}
      </div>
      <div className="row-cell text-cell trans-text" title={item.translation}>
        {item.translation || "—"}
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
  const useRegex = useAppStore((s) => s.useRegex);
  const replaceText = useAppStore((s) => s.replaceText);
  const statusFilter = useAppStore((s) => s.statusFilter);
  const selectedId = useAppStore((s) => s.selectedId);
  const total = useAppStore((s) => s.total);
  const filtered = useAppStore((s) => s.filtered);

  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const setFilter = useAppStore((s) => s.setFilter);
  const setUseRegex = useAppStore((s) => s.setUseRegex);
  const setReplaceText = useAppStore((s) => s.setReplaceText);
  const setSort = useAppStore((s) => s.setSort);
  const setStatusFilter = useAppStore((s) => s.setStatusFilter);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const selectNextRow = useAppStore((s) => s.selectNextRow);
  const selectPrevRow = useAppStore((s) => s.selectPrevRow);
  const replaceAll = useAppStore((s) => s.replaceAll);

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
          <Spinner size={24} />
          <span>{t("table.loading")}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="string-table-wrapper">
      <div className="table-toolbar">
        <div className="table-toolbar-row">
          <Input
            size="sm"
            icon={<Search size={14} />}
            placeholder={useRegex ? t("common.regexFilter") : t("common.filter")}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            suffix={
              <Button
                variant="ghost"
                size="xs"
                onClick={() => setUseRegex(!useRegex)}
                title={useRegex ? t("table.regexSwitchTip") : t("table.plainSwitchTip")}
                active={useRegex}
              >
                <Code2 size={14} />
              </Button>
            }
          />
          <div className="status-filters">
            {[
              { key: null, label: t("common.all") },
              { key: "incomplete", label: t("common.incomplete") },
              { key: "translated", label: t("common.translated") },
              { key: "locked", label: t("common.locked") },
            ].map((s) => (
              <Button
                key={s.label}
                variant="ghost"
                size="sm"
                active={statusFilter === s.key}
                onClick={() => setStatusFilter(statusFilter === s.key ? null : (s.key as string))}
              >
                {s.label}
              </Button>
            ))}
          </div>
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
        <div className="header-cell" style={{ width: 60 }} onClick={() => handleSort("id")}>
          ID <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ width: 80 }}>{t("table.status")}</div>
        <div className="header-cell" style={{ width: 80 }} onClick={() => handleSort("record_sig")}>
          Rec <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ width: 80 }}>{t("table.field")}</div>
        <div className="header-cell" style={{ width: 100 }}>{t("table.formId")}</div>
        <div className="header-cell" style={{ flex: 1 }}>{t("table.source")}</div>
        <div className="header-cell" style={{ flex: 1 }}>{t("table.translation")}</div>
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
    </div>
  );
}
