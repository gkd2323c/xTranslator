import { useState, useMemo } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { FileCode, FileUp, Code, Search, X, Copy } from "lucide-react";
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
  const [stringSearch, setStringSearch] = useState("");
  const [expandedStrings, setExpandedStrings] = useState<Set<number>>(new Set());
  const language = useAppStore((s) => s.language);

  const types = [...new Set((script?.translatable ?? []).map((t) => t.string_type))];

  // 搜索 + 类型过滤后的字符串
  const filtered = useMemo(() => {
    let result = script?.translatable ?? [];
    if (selectedType) {
      result = result.filter((t) => t.string_type === selectedType);
    }
    if (stringSearch) {
      const q = stringSearch.toLowerCase();
      result = result.filter(
        (t) =>
          t.source_text.toLowerCase().includes(q) ||
          t.object_name.toLowerCase().includes(q) ||
          (t.function_name && t.function_name.toLowerCase().includes(q))
      );
    }
    return result;
  }, [script, selectedType, stringSearch]);

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
    setStringSearch("");
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

  const toggleStringExpand = (idx: number) => {
    setExpandedStrings((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx); else next.add(idx);
      return next;
    });
  };

  /** 搜索高亮 */
  function highlightText(text: string, query: string): string {
    if (!query) return text;
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
          {/* Script Info + Actions */}
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

          {/* ── Strings View ── */}
          {view === "strings" && (
            <>
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

              {/* Search bar */}
              <div className="sidepanel-section" style={{ padding: "4px 8px" }}>
                <div className="pex-search-bar">
                  <Search size={12} className="pex-search-icon" />
                  <input
                    type="text"
                    className="pex-search-input"
                    placeholder={t("pex.searchStrings", { defaultValue: "Search strings..." })}
                    value={stringSearch}
                    onChange={(e) => setStringSearch(e.target.value)}
                  />
                  {stringSearch && (
                    <button className="pex-search-clear" onClick={() => setStringSearch("")}>
                      <X size={12} />
                    </button>
                  )}
                </div>
              </div>

              {/* String table */}
              <div className="sidepanel-section">
                <h3>
                  {t("pex.stringsCount", { count: filtered.length })}
                  {(selectedType || stringSearch) && ` (${script.translatable.length})`}
                </h3>
                <div className="pex-string-table">
                  {filtered.length === 0 ? (
                    <div className="pex-string-empty">{t("pex.noMatch", { defaultValue: "No matching strings" })}</div>
                  ) : (
                    filtered.map((entry) => {
                      const idx = script.translatable.indexOf(entry);
                      const isExpanded = expandedStrings.has(idx);
                      const fullPath = [entry.object_name, entry.state_name && `::${entry.state_name}`, entry.function_name && `.${entry.function_name}`].filter(Boolean).join("");
                      return (
                        <div
                          key={idx}
                          className={`pex-string-row ${isExpanded ? "pex-string-expanded" : ""}`}
                          onClick={() => toggleStringExpand(idx)}
                        >
                          <div className="pex-string-header">
                            <span className="pex-string-type">
                              <Badge variant="incomplete" size="sm">{entry.string_type}</Badge>
                            </span>
                            <span className="pex-string-path" title={fullPath}>
                              {stringSearch ? (
                                <span dangerouslySetInnerHTML={{ __html: highlightText(fullPath, stringSearch) }} />
                              ) : (
                                fullPath
                              )}
                            </span>
                          </div>
                          <div className="pex-string-source">
                            {stringSearch ? (
                              <span dangerouslySetInnerHTML={{ __html: highlightText(entry.source_text, stringSearch) }} />
                            ) : (
                              entry.source_text.slice(0, isExpanded ? undefined : 100)
                            )}
                            {!isExpanded && entry.source_text.length > 100 && "..."}
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>
              </div>
            </>
          )}

          {/* ── Pseudocode View ── */}
          {view === "decompile" && decompiled && (
            <div className="sidepanel-section">
              <div className="pex-decompile-header">
                <h3>{t("pex.pseudocodeTitle", { defaultValue: "Pseudocode" })}</h3>
                <div className="pex-decompile-actions">
                  <span className="pex-decompile-stats">
                    {decompiled.object_count} obj · {decompiled.function_count} fn · {decompiled.instruction_count} inst
                  </span>
                  <button
                    className="pex-decompile-copy"
                    onClick={() => navigator.clipboard.writeText(decompiled.pseudocode)}
                    title={t("pex.copyCode", { defaultValue: "Copy code" })}
                  >
                    <Copy size={12} />
                  </button>
                </div>
              </div>
              <pre className="pex-pseudocode">
                {decompiled.pseudocode.split("\n").map((line, i) => (
                  <span key={i} className="pex-pseudocode-line">
                    <span className="pex-pseudocode-line-num">{i + 1}</span>
                    <span className="pex-pseudocode-line-text">{line || " "}</span>
                  </span>
                ))}
              </pre>
            </div>
          )}
        </>
      )}
    </div>
  );
}
