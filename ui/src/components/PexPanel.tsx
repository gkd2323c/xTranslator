import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { FileCode, FileUp, Code } from "lucide-react";
import toast from "react-hot-toast";
import { parsePexStrings, exportXml, decompilePex } from "../api/strings";
import type { PexScriptDto, DecompilePexResponse } from "../api/strings";
import { useAppStore } from "../stores/appStore";
import { Button, EmptyState, Badge } from "./ui";

export function PexPanel() {
  const { t } = useTranslation();
  const [script, setScript] = useState<PexScriptDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedType, setSelectedType] = useState<string | null>(null);
  const [view, setView] = useState<"strings" | "decompile">("strings");
  const [decompiled, setDecompiled] = useState<DecompilePexResponse | null>(null);
  const [decompiling, setDecompiling] = useState(false);
  const [pexPath, setPexPath] = useState<string>("");
  const language = useAppStore((s) => s.language);

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

    setPexPath(path as string);
    setDecompiled(null);
    setView("strings");
    setLoading(true);
    try {
      const result = await parsePexStrings(path, language === "english" ? "SkyrimSE" : undefined);
      setScript(result);
      toast.success(t("pex.foundStrings", { count: result.translatable.length }));
    } catch (e: any) {
      toast.error(`${t("pex.parseFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDecompile = async () => {
    if (!pexPath) return;
    setDecompiling(true);
    try {
      const result = await decompilePex(pexPath);
      setDecompiled(result);
      setView("decompile");
      toast.success(
        t("pex.decompiled", {
          defaultValue: "Decompiled: {{funcs}} functions, {{insts}} instructions",
          funcs: result.function_count,
          insts: result.instruction_count,
        })
      );
    } catch (e: any) {
      toast.error(`${t("pex.decompileFailed", { defaultValue: "Decompile failed" })}: ${e}`);
    } finally {
      setDecompiling(false);
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
      toast.success(t("pex.exportedEntries", { count }));
    } catch (e: any) {
      toast.error(`${t("pex.exportFailed")}: ${e}`);
    }
  };

  return (
    <div className="sidepanel">
      {!script ? (
        <div className="sidepanel-empty">
          <EmptyState
            icon={<FileCode size={36} />}
            title={t("pex.title")}
            hint={t("pex.subtitle")}
          />
          <Button variant="primary" onClick={handleOpen} disabled={loading} icon={<FileUp size={16} />} className="pex-open-btn">
            {loading ? t("pex.parsing") : t("pex.openPex")}
          </Button>
        </div>
      ) : (
        <>
          <div className="sidepanel-section">
            <h3>{t("pex.scriptInfo")}</h3>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("pex.name")}</span>
              <span className="sidepanel-value">{script.script_name}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("pex.version")}</span>
              <span className="sidepanel-value">
                {script.major_version}.{script.minor_version}
              </span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("pex.stringsHeader")}</span>
              <span className="sidepanel-value">
                {t("pex.stringsDetail", { tableCount: script.string_count, transCount: script.translatable.length })}
              </span>
            </div>
            <div className="pex-action-buttons">
              <Button variant="default" size="sm" onClick={handleOpen} icon={<FileUp size={12} />}>
                {t("pex.openAnother")}
              </Button>
              <Button variant="default" size="sm" onClick={handleExportXml} icon={<FileCode size={12} />} disabled={script.translatable.length === 0}>
                {t("pex.exportXml")}
              </Button>
              <Button
                variant="default"
                size="sm"
                onClick={handleDecompile}
                icon={<Code size={12} />}
                disabled={decompiling}
              >
                {decompiling
                  ? t("pex.decompiling", { defaultValue: "Decompiling..." })
                  : t("pex.decompile", { defaultValue: "Decompile" })}
              </Button>
            </div>

            {/* View toggle */}
            {decompiled && (
              <div className="pex-view-toggle" style={{ display: "flex", gap: 8, marginTop: 8 }}>
                <Button
                  variant={view === "strings" ? "primary" : "default"}
                  size="sm"
                  onClick={() => setView("strings")}
                >
                  {t("pex.stringsView", { defaultValue: "Strings" })}
                </Button>
                <Button
                  variant={view === "decompile" ? "primary" : "default"}
                  size="sm"
                  onClick={() => setView("decompile")}
                >
                  {t("pex.pseudocodeView", { defaultValue: "Pseudocode" })}
                </Button>
              </div>
            )}
          </div>

          {/* Type filter */}
          <div className="sidepanel-section">
            <h3>{t("pex.stringTypes")}</h3>
            <div className="record-type-row" onClick={() => setSelectedType(null)}>
              <span className="sidepanel-label">{t("pex.all")}</span>
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

          {/* String list — shown in strings view */}
          {view === "strings" && (
            <div className="sidepanel-section">
              <h3>{t("pex.stringsCount", { count: filtered.length })}</h3>
              <div style={{ maxHeight: 300, overflowY: "auto" }}>
                {filtered.map((entry, i) => (
                  <div key={i} className="record-type-row pex-string-entry">
                    <div className="pex-string-path">
                      {entry.object_name}
                      {entry.state_name && ` :: ${entry.state_name}`}
                      {entry.function_name && `.${entry.function_name}`}
                    </div>
                    <div className="pex-string-text">
                      {entry.source_text}
                    </div>
                    <Badge variant="incomplete" size="sm">
                      {entry.string_type}
                    </Badge>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Pseudocode view */}
          {view === "decompile" && decompiled && (
            <div className="sidepanel-section">
              <h3>{t("pex.pseudocodeTitle", { defaultValue: "Pseudocode" })}</h3>
              <div style={{ marginBottom: 8, fontSize: 12, opacity: 0.7 }}>
                {decompiled.object_count} object(s), {decompiled.function_count} function(s), {decompiled.instruction_count} instruction(s)
              </div>
              <pre
                style={{
                  maxHeight: 500,
                  overflow: "auto",
                  fontSize: 12,
                  lineHeight: 1.5,
                  background: "var(--bg-secondary, #1e1e1e)",
                  color: "var(--text-primary, #d4d4d4)",
                  padding: 12,
                  borderRadius: 6,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                }}
              >
                {decompiled.pseudocode}
              </pre>
            </div>
          )}
        </>
      )}
    </div>
  );
}
