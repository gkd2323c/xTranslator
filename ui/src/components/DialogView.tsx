import { useState } from "react";
import { useTranslation } from "react-i18next";
import { MessagesSquare, ChevronDown, ChevronRight } from "lucide-react";
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
  const setFilter = useAppStore((s) => s.setFilter);
  const setSelectedById = useAppStore((s) => s.setSelectedById);

  const handleLoad = async () => {
    setLoading(true);
    try {
      const result = await buildDialogTree();
      setTree(result);
      setExpanded(new Set());
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

  const handleSelect = (entry: DialogInfoDto) => {
    setSelectedById(entry.id);
    setFilter(String(entry.form_id));
  };

  return (
    <div className="sidepanel">
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
          <div className="sidepanel-section">
            <h3>{t("dialog.dialogGroupsCount", { count: tree.npcs.length })}</h3>
            <Button variant="default" size="sm" onClick={handleLoad} className="dialog-reload-btn">
              {t("dialog.reload")}
            </Button>
          </div>
          <div style={{ maxHeight: 500, overflowY: "auto" }}>
            {tree.npcs.map((group) => {
              const isOpen = expanded.has(group.npc_edid);
              const translated = group.dialogues.filter((d) => d.translation).length;
              return (
                <div key={group.npc_edid}>
                  <div
                    className="record-type-row dialog-group-row"
                    onClick={() => toggleGroup(group.npc_edid)}
                  >
                    {isOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    <MessagesSquare size={12} className="dialog-group-icon" />
                    <span className="sidepanel-label dialog-group-label">
                      {group.npc_edid.replace("DIAL_", "")}
                    </span>
                    <span className="sidepanel-value dialog-group-count">
                      {translated}/{group.dialogues.length}
                    </span>
                  </div>
                  {isOpen && (
                    <div className="dialog-entries">
                      {group.dialogues.map((d) => (
                        <div
                          key={d.id}
                          className="record-type-row dialog-entry"
                          onClick={() => handleSelect(d)}
                        >
                          <div className={d.translation ? "dialog-entry-text translated" : "dialog-entry-text"}>
                            {d.dialog_text.slice(0, 80)}
                            {d.dialog_text.length > 80 && "..."}
                          </div>
                          {d.translation && (
                            <div className="dialog-entry-translation">
                              → {d.translation.slice(0, 60)}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}
