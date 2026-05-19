import { useState, useMemo, useRef, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ScrollText, Search, X, Trash2, Copy, ArrowDownToLine } from "lucide-react";
import { useAppStore } from "../../stores/appStore";
import { EmptyState } from "../ui";

export function LogPanel() {
  const { t } = useTranslation();
  const logs = useAppStore((s) => s.logs);
  const clearLogs = useAppStore((s) => s.clearLogs);
  const [search, setSearch] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);
  const listRef = useRef<HTMLDivElement>(null);

  const filtered = useMemo(() => {
    if (!search) return logs;
    const q = search.toLowerCase();
    return logs.filter(
      (e) => e.message.toLowerCase().includes(q) || (e.source && e.source.toLowerCase().includes(q))
    );
  }, [logs, search]);

  // 自动滚动到底部
  useEffect(() => {
    if (autoScroll && listRef.current) {
      listRef.current.scrollTop = 0;
    }
  }, [filtered.length, autoScroll]);

  const levelIcon = (level: string) => {
    switch (level) {
      case "error": return "✕";
      case "warn": return "⚠";
      default: return "·";
    }
  };

  return (
    <div className="bottom-panel-inner">
      {logs.length === 0 ? (
        <EmptyState
          icon={<ScrollText size={32} />}
          title={t("bottomTabs.log")}
          hint={t("logPanel.hint")}
        />
      ) : (
        <div className="log-content">
          {/* 工具栏 */}
          <div className="log-toolbar">
            <div className="log-search-bar">
              <Search size={13} className="log-search-icon" />
              <input
                type="text"
                className="log-search-input"
                placeholder={t("logPanel.searchPlaceholder", { defaultValue: "Filter logs..." })}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              {search && (
                <button className="log-search-clear" onClick={() => setSearch("")}>
                  <X size={13} />
                </button>
              )}
            </div>
            <div className="log-actions">
              <button
                className={`log-action-btn ${autoScroll ? "log-action-active" : ""}`}
                onClick={() => setAutoScroll(!autoScroll)}
                title={t("logPanel.autoScroll", { defaultValue: "Auto-scroll" })}
              >
                <ArrowDownToLine size={14} />
              </button>
              <button
                className="log-action-btn"
                onClick={() => {
                  const text = filtered.map((e) => `[${e.level.toUpperCase()}] ${e.message}`).join("\n");
                  navigator.clipboard.writeText(text);
                }}
                title={t("logPanel.copyAll", { defaultValue: "Copy all" })}
              >
                <Copy size={14} />
              </button>
              <button
                className="log-action-btn"
                onClick={clearLogs}
                title={t("logPanel.clear", { defaultValue: "Clear logs" })}
              >
                <Trash2 size={14} />
              </button>
            </div>
          </div>

          {/* 日志条目计数 */}
          <div className="log-count">
            {search ? `${filtered.length} / ${logs.length}` : `${logs.length} entries`}
          </div>

          {/* 日志列表 */}
          <div className="log-list" ref={listRef}>
            {filtered.map((entry) => (
              <div key={entry.id} className={`log-entry log-level-${entry.level}`}>
                <span className="log-entry-icon">{levelIcon(entry.level)}</span>
                <span className="log-entry-msg">{entry.message}</span>
                {entry.source && <span className="log-entry-source">{entry.source}</span>}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
