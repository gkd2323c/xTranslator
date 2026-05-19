import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { MessagesSquare, ChevronDown, ChevronRight, Search, X, Maximize2, Minimize2 } from "lucide-react";
import toast from "react-hot-toast";
import { useAppStore } from "../stores/appStore";
import { buildDialogTree } from "../api/strings";
import type { DialogTreeDto, DialogInfoDto } from "../api/strings";
import { Button, EmptyState } from "./ui";

export function DialogView() {
  const { t } = useTranslation();
  const [tree, setTree] = useState<DialogTreeDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const setFilter = useAppStore((s) => s.setFilter);
  const setSelectedById = useAppStore((s) => s.setSelectedById);

  const handleLoad = async () => {
    setLoading(true);
    try {
      const result = await buildDialogTree();
      setTree(result);
      setExpanded(new Set());
      setSearch("");
      toast.success(t("dialog.loaded", { count: result.npcs.length }));
    } catch (e: any) {
      toast.error(`${t("dialog.loadFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const toggleGroup = (key: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  };

  // 搜索过滤：过滤对话条目（仅展开匹配的组）
  const filteredNpcs = useMemo(() => {
    if (!search) return tree ? tree.npcs : [];
    const q = search.toLowerCase();
    return tree!.npcs
      .map((group) => {
        const matchingDialogues = group.dialogues.filter(
          (d) =>
            d.dialog_text.toLowerCase().includes(q) ||
            (d.translation && d.translation.toLowerCase().includes(q))
        );
        return { ...group, dialogues: matchingDialogues };
      })
      .filter((group) => group.dialogues.length > 0);
  }, [tree, search]);

  // 搜索时自动展开所有匹配组
  const effectiveExpanded = useMemo(() => {
    if (!search) return expanded;
    const all = new Set(expanded);
    filteredNpcs.forEach((g) => all.add(g.npc_edid));
    return all;
  }, [expanded, filteredNpcs, search]);

  const expandAll = () => {
    if (!tree) return;
    setExpanded(new Set(tree.npcs.map((g) => g.npc_edid)));
  };

  const collapseAll = () => {
    setExpanded(new Set());
  };

  const handleSelect = (entry: DialogInfoDto) => {
    setSelectedById(entry.id);
    setFilter(String(entry.form_id));
  };

  return (
    <div className="dialog-view">
      {!tree ? (
        <div className="sidepanel-empty">
          <EmptyState
            icon={<MessagesSquare size={36} />}
            title={t("dialog.title")}
            hint={t("dialog.subtitle")}
          />
          <Button variant="primary" onClick={handleLoad} disabled={loading} icon={<MessagesSquare size={16} />} className="dialog-load-btn">
            {loading ? t("dialog.loading") : t("dialog.loadDialogs")}
          </Button>
        </div>
      ) : (
        <>
          {/* 头部：统计 + 操作按钮 */}
          <div className="dialog-view-header">
            <div className="dialog-view-stats">
              <span className="dialog-view-stat-label">{t("dialog.dialogGroupsCount", { count: tree.npcs.length })}</span>
            </div>
            <div className="dialog-view-actions">
              <Button variant="ghost" size="xs" onClick={expandAll} title={t("dialog.expandAll", { defaultValue: "Expand All" })}>
                <Maximize2 size={12} />
              </Button>
              <Button variant="ghost" size="xs" onClick={collapseAll} title={t("dialog.collapseAll", { defaultValue: "Collapse All" })}>
                <Minimize2 size={12} />
              </Button>
              <Button variant="default" size="xs" onClick={handleLoad} disabled={loading} className="dialog-reload-btn">
                {t("dialog.reload")}
              </Button>
            </div>
          </div>

          {/* 搜索框 */}
          <div className="dialog-view-search">
            <Search size={13} className="dialog-search-icon" />
            <input
              type="text"
              className="dialog-search-input"
              placeholder={t("dialog.searchPlaceholder", { defaultValue: "Filter dialogues..." })}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
            {search && (
              <button className="dialog-search-clear" onClick={() => setSearch("")}>
                <X size={13} />
              </button>
            )}
          </div>

          {/* NPC 对话树列表 */}
          <div className="dialog-view-list">
            {(search ? filteredNpcs : tree.npcs).map((group) => {
              const isOpen = effectiveExpanded.has(group.npc_edid);
              const translated = group.dialogues.filter((d) => d.translation).length;
              return (
                <div key={group.npc_edid}>
                  <div
                    className="dialog-group-row"
                    onClick={() => toggleGroup(group.npc_edid)}
                  >
                    {isOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    <MessagesSquare size={12} className="dialog-group-icon" />
                    <span className="dialog-group-label">
                      {group.npc_edid.replace("DIAL_", "")}
                    </span>
                    <span className="dialog-group-count" data-status={translated === group.dialogues.length ? "done" : translated > 0 ? "partial" : "none"}>
                      {translated}/{group.dialogues.length}
                    </span>
                  </div>
                  {isOpen && (
                    <div className="dialog-entries">
                      {group.dialogues.map((d) => (
                        <div
                          key={d.id}
                          className={`dialog-entry ${d.translation ? "dialog-entry-done" : "dialog-entry-pending"}`}
                          onClick={() => handleSelect(d)}
                        >
                          <div className="dialog-entry-text-wrapper">
                            <div className={`dialog-entry-text ${d.translation ? "translated" : ""}`}>
                              {search
                                ? highlightMatch(d.dialog_text, search)
                                : d.dialog_text.slice(0, 80)}
                              {d.dialog_text.length > 80 && "..."}
                            </div>
                            {d.translation && (
                              <div className="dialog-entry-translation">
                                → {search
                                  ? highlightMatch(d.translation.slice(0, 60), search)
                                  : d.translation.slice(0, 60)}
                              </div>
                            )}
                          </div>
                          <span className={`dialog-entry-status ${d.translation ? "status-translated" : "status-incomplete"}`}>
                            {d.translation ? "●" : "○"}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
            {search && filteredNpcs.length === 0 && (
              <div className="dialog-view-empty">{t("dialog.noSearchResults", { defaultValue: "No matching dialogues" })}</div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

/** 简易搜索高亮 */
function highlightMatch(text: string, query: string): string {
  if (!query) return text.slice(0, 80);
  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const parts = text.split(new RegExp(`(${escaped})`, "gi"));
  return parts
    .map((part) =>
      part.toLowerCase() === query.toLowerCase()
        ? `<mark>${part}</mark>`
        : part
    )
    .join("");
}