import { useState } from "react";
import { MessagesSquare, ChevronDown, ChevronRight } from "lucide-react";
import toast from "react-hot-toast";
import { useAppStore } from "../stores/appStore";
import { buildDialogTree } from "../api/strings";
import type { DialogTreeDto, DialogInfoDto } from "../api/strings";

export function DialogView() {
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
      toast.success(`${result.npcs.length} dialog groups loaded`);
    } catch (e: any) {
      toast.error(`Failed: ${e}`);
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
          <MessagesSquare size={36} />
          <p style={{ marginTop: 8 }}>Dialog View</p>
          <p className="sidepanel-hint">Group dialogues by parent DIAL</p>
          <button onClick={handleLoad} disabled={loading} className="btn btn-primary" style={{ marginTop: 16 }}>
            <MessagesSquare size={16} />
            <span>{loading ? "Loading..." : "Load Dialogs"}</span>
          </button>
        </div>
      ) : (
        <>
          <div className="sidepanel-section">
            <h3>Dialog Groups ({tree.npcs.length})</h3>
            <button onClick={handleLoad} className="btn btn-sm" style={{ marginBottom: 8, width: "100%" }}>
              Reload
            </button>
          </div>
          <div style={{ maxHeight: 500, overflowY: "auto" }}>
            {tree.npcs.map((group) => {
              const isOpen = expanded.has(group.npc_edid);
              const translated = group.dialogues.filter((d) => d.translation).length;
              return (
                <div key={group.npc_edid}>
                  <div
                    className="record-type-row"
                    style={{ display: "flex", alignItems: "center", gap: 4 }}
                    onClick={() => toggleGroup(group.npc_edid)}
                  >
                    {isOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    <MessagesSquare size={12} style={{ color: "var(--accent-gold)", flexShrink: 0 }} />
                    <span className="sidepanel-label" style={{ flex: 1, fontSize: 10, fontFamily: "monospace" }}>
                      {group.npc_edid.replace("DIAL_", "")}
                    </span>
                    <span className="sidepanel-value" style={{ fontSize: 10 }}>
                      {translated}/{group.dialogues.length}
                    </span>
                  </div>
                  {isOpen && (
                    <div style={{ marginLeft: 16, marginBottom: 4 }}>
                      {group.dialogues.map((d) => (
                        <div
                          key={d.id}
                          className="record-type-row"
                          style={{ padding: "4px 8px", lineHeight: 1.3, cursor: "pointer" }}
                          onClick={() => handleSelect(d)}
                        >
                          <div
                            style={{
                              fontSize: 11,
                              color: d.translation ? "var(--success)" : "var(--text-primary)",
                            }}
                          >
                            {d.dialog_text.slice(0, 80)}
                            {d.dialog_text.length > 80 && "..."}
                          </div>
                          {d.translation && (
                            <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 1 }}>
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
