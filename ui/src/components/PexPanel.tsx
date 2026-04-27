import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { FileCode, FileUp } from "lucide-react";
import toast from "react-hot-toast";
import { parsePexStrings, exportXml } from "../api/strings";
import type { PexScriptDto } from "../api/strings";

export function PexPanel() {
  const [script, setScript] = useState<PexScriptDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedType, setSelectedType] = useState<string | null>(null);

  const types = [...new Set((script?.translatable ?? []).map((t) => t.string_type))];
  const filtered = selectedType
    ? script?.translatable.filter((t) => t.string_type === selectedType) ?? []
    : script?.translatable ?? [];

  const handleOpen = async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "Papyrus Script", extensions: ["pex"] },
        { name: "All", extensions: ["*"] },
      ],
    });
    if (!path) return;

    setLoading(true);
    try {
      const result = await parsePexStrings(path);
      setScript(result);
      toast.success(`${result.translatable.length} translatable strings found`);
    } catch (e: any) {
      toast.error(`Failed to parse: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleExportXml = async () => {
    if (!script) return;
    const path = await save({
      filters: [{ name: "XML Export", extensions: ["xml"] }],
      defaultPath: `${script.script_name}_pex_strings.xml`,
    });
    if (!path) return;

    try {
      const count = await exportXml({ path, dest_lang: "chinese" });
      toast.success(`Exported ${count} entries`);
    } catch (e: any) {
      toast.error(`Export failed: ${e}`);
    }
  };

  return (
    <div className="sidepanel">
      {!script ? (
        <div className="sidepanel-empty">
          <FileCode size={36} />
          <p style={{ marginTop: 8 }}>Open a PEX script</p>
          <p className="sidepanel-hint">Extract translatable strings</p>
          <button onClick={handleOpen} disabled={loading} className="btn btn-primary" style={{ marginTop: 16 }}>
            <FileUp size={16} />
            <span>{loading ? "Parsing..." : "Open PEX"}</span>
          </button>
        </div>
      ) : (
        <>
          <div className="sidepanel-section">
            <h3>Script Info</h3>
            <div className="sidepanel-row">
              <span className="sidepanel-label">Name</span>
              <span className="sidepanel-value">{script.script_name}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">Version</span>
              <span className="sidepanel-value">
                {script.major_version}.{script.minor_version}
              </span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">Strings</span>
              <span className="sidepanel-value">{script.string_count} (table) / {script.translatable.length} (translatable)</span>
            </div>
            <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
              <button onClick={handleOpen} className="btn btn-sm" style={{ flex: 1 }}>
                <FileUp size={12} /> Open Another
              </button>
              <button onClick={handleExportXml} className="btn btn-sm" style={{ flex: 1 }} disabled={script.translatable.length === 0}>
                <FileCode size={12} /> Export XML
              </button>
            </div>
          </div>

          {/* Type filter */}
          <div className="sidepanel-section">
            <h3>String Types</h3>
            <div className="record-type-row" onClick={() => setSelectedType(null)}>
              <span className="sidepanel-label">All</span>
              <span className="sidepanel-value">{script.translatable.length}</span>
            </div>
            {types.map((t) => {
              const count = script.translatable.filter((x) => x.string_type === t).length;
              return (
                <div
                  key={t}
                  className={`record-type-row ${selectedType === t ? "active" : ""}`}
                  onClick={() => setSelectedType(selectedType === t ? null : t)}
                >
                  <span className="sidepanel-label">{t}</span>
                  <span className="sidepanel-value">{count}</span>
                </div>
              );
            })}
          </div>

          {/* String list */}
          <div className="sidepanel-section">
            <h3>Strings ({filtered.length})</h3>
            <div style={{ maxHeight: 300, overflowY: "auto" }}>
              {filtered.map((entry, i) => (
                <div key={i} className="record-type-row" style={{ padding: "6px 8px", lineHeight: 1.4 }}>
                  <div style={{ fontSize: 10, color: "var(--text-muted)" }}>
                    {entry.object_name}
                    {entry.state_name && ` :: ${entry.state_name}`}
                    {entry.function_name && `.${entry.function_name}`}
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-primary)", marginTop: 2 }}>
                    {entry.source_text}
                  </div>
                  <div className="badge badge-incomplete" style={{ marginTop: 2, fontSize: 8 }}>
                    {entry.string_type}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
