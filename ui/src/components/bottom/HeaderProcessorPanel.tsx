import { useState, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  headerRulesLoad,
  headerRulesList,
  headerRulesToggle,
  headerRulesApply,
  headerRulesSave,
  headerRulesDelete,
  headerRulesMove,
  headerRulesUpdate,
  headerRulesAdd,
  headerTemplatesList,
  headerTemplatesSave,
  headerTemplatesLoad,
  headerTemplatesDelete,
  preprocOptsLoad,
  preprocOptsSet,
  preprocOptsDelete,
  preprocOptsSave,
  type HeaderRuleDto,
  type HeaderApplyResult,
  type TemplateInfo,
} from "../../api/strings";
import toast from "react-hot-toast";
import { Button } from "../ui";
import { Plus, Trash2, ChevronUp, ChevronDown, Save, Search, FolderOpen } from "lucide-react";
import { useAppStore } from "../../stores/appStore";

function editableField(
  value: string,
  placeholder: string,
  onChange: (val: string) => void,
  style?: React.CSSProperties,
) {
  return (
    <input
      className="ui-input"
      style={{ fontSize: "12px", padding: "1px 4px", width: "100%", ...style }}
      value={value}
      placeholder={placeholder}
      onChange={(e) => onChange(e.target.value)}
      onClick={(e) => e.stopPropagation()}
    />
  );
}

export function HeaderProcessorPanel() {
  const { t } = useTranslation();
  const currentGame = useAppStore((s) => s.currentGame);
  const [rules, setRules] = useState<HeaderRuleDto[]>([]);
  const [result, setResult] = useState<HeaderApplyResult | null>(null);
  const [filePath, setFilePath] = useState("");
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState("");
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [editing, setEditing] = useState<{ index: number; field: string } | null>(null);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [templateDir, setTemplateDir] = useState("");
  const [templateName, setTemplateName] = useState("");
  const [opts, setOpts] = useState<[string, string][]>([]);
  const [optsPath, setOptsPath] = useState("");
  const [newOptKey, setNewOptKey] = useState("");
  const [newOptVal, setNewOptVal] = useState("");

  useEffect(() => {
    headerRulesList().then(setRules).catch(() => {});
  }, []);

  const filteredRules = useMemo(() => {
    if (!filter) return rules;
    const q = filter.toLowerCase();
    return rules.filter(
      (r) =>
        (r.header || "").toLowerCase().includes(q) ||
        r.r_sig.toLowerCase().includes(q) ||
        r.f_sig.toLowerCase().includes(q) ||
        r.in_edid.some((e) => e.toLowerCase().includes(q)) ||
        r.include_keywords.some((k) => k.name.toLowerCase().includes(q)),
    );
  }, [rules, filter]);

  const loadFile = async () => {
    if (!filePath) return;
    setLoading(true);
    try {
      const loaded = await headerRulesLoad(filePath);
      setRules(loaded);
      toast.success(t("headerProcessor.loadedRules", { count: loaded.length }));
    } catch (e: any) {
      toast.error(t("headerProcessor.loadFailed", { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  const toggleRule = async (index: number, enabled: boolean) => {
    try {
      await headerRulesToggle(index, enabled);
      setRules((prev) => prev.map((r) => (r.index === index ? { ...r, enabled } : r)));
    } catch (e: any) {
      toast.error(t("headerProcessor.toggleFailed", { error: String(e) }));
    }
  };

  const applyRules = async () => {
    if (!currentGame) {
      toast.error(t("headerProcessor.selectGame", { defaultValue: "Select or load a game workspace first." }));
      return;
    }
    setLoading(true);
    try {
      const res = await headerRulesApply();
      setResult(res);
      toast.success(t("headerProcessor.applyResult", {
        matched: res.strings_matched,
        enabled: res.enabled_rules,
        total: res.total_rules,
      }));
    } catch (e: any) {
      toast.error(t("headerProcessor.applyFailed", { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  const saveRules = async () => {
    if (!filePath) return;
    setLoading(true);
    try {
      await headerRulesSave(filePath);
      toast.success(t("headerProcessor.savedRules", { count: rules.length }));
    } catch (e: any) {
      toast.error(t("headerProcessor.saveFailed", { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  const deleteRule = async (index: number) => {
    try {
      const updated = await headerRulesDelete(index);
      setRules(updated);
      toast.success(t("headerProcessor.ruleDeleted"));
    } catch (e: any) {
      toast.error(t("headerProcessor.deleteFailed", { error: String(e) }));
    }
  };

  const moveRule = async (index: number, dir: "up" | "down") => {
    try {
      const updated = await headerRulesMove(index, dir);
      setRules(updated);
    } catch (e: any) {
      toast.error(t("headerProcessor.moveFailed", { error: String(e) }));
    }
  };

  const addRule = async () => {
    try {
      const updated = await headerRulesAdd();
      setRules(updated);
      toast.success(t("headerProcessor.ruleAdded"));
    } catch (e: any) {
      toast.error(t("headerProcessor.addFailed", { error: String(e) }));
    }
  };

  const updateField = async (index: number, field: string, value: string) => {
    setEditing(null);
    try {
      const updated = await headerRulesUpdate(index, field, value);
      setRules(updated);
    } catch (e: any) {
      toast.error(t("headerProcessor.updateFailed", { error: String(e) }));
    }
  };

  const toggleExpanded = (index: number) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  const refreshTemplates = async () => {
    if (!templateDir) return;
    try {
      const list = await headerTemplatesList(templateDir);
      setTemplates(list);
    } catch { /* dir may not exist yet */ }
  };

  const saveTemplate = async () => {
    if (!templateDir || !templateName) return;
    try {
      await headerTemplatesSave(templateDir, templateName);
      toast.success(t("headerProcessor.templateSaved", { name: templateName }));
      setTemplateName("");
      refreshTemplates();
    } catch (e: any) {
      toast.error(t("headerProcessor.saveTemplateFailed", { error: String(e) }));
    }
  };

  const loadTemplate = async (name: string) => {
    if (!templateDir) return;
    try {
      const loaded = await headerTemplatesLoad(templateDir, name);
      setRules(loaded);
      toast.success(t("headerProcessor.templateLoaded", { name, count: loaded.length }));
    } catch (e: any) {
      toast.error(t("headerProcessor.loadTemplateFailed", { error: String(e) }));
    }
  };

  const deleteTemplate = async (name: string) => {
    if (!templateDir) return;
    try {
      await headerTemplatesDelete(templateDir, name);
      toast.success(t("headerProcessor.templateDeleted", { name }));
      refreshTemplates();
    } catch (e: any) {
      toast.error(t("headerProcessor.deleteTemplateFailed", { error: String(e) }));
    }
  };

  const loadOpts = async () => {
    if (!optsPath) return;
    try {
      const dto = await preprocOptsLoad(optsPath);
      setOpts(dto.options);
      toast.success(t("headerProcessor.loadedOptions", { count: dto.options.length }));
    } catch (e: any) {
      toast.error(t("headerProcessor.loadOptionsFailed", { error: String(e) }));
    }
  };

  const saveOpts = async () => {
    if (!optsPath) return;
    try {
      await preprocOptsSave(optsPath);
      toast.success(t("headerProcessor.optionsSaved"));
    } catch (e: any) {
      toast.error(t("headerProcessor.saveOptionsFailed", { error: String(e) }));
    }
  };

  const addOpt = async () => {
    if (!newOptKey) return;
    try {
      const dto = await preprocOptsSet(newOptKey, newOptVal);
      setOpts(dto.options);
      setNewOptKey("");
      setNewOptVal("");
    } catch (e: any) {
      toast.error(t("headerProcessor.setOptionFailed", { error: String(e) }));
    }
  };

  const deleteOpt = async (key: string) => {
    try {
      const dto = await preprocOptsDelete(key);
      setOpts(dto.options);
    } catch (e: any) {
      toast.error(t("headerProcessor.deleteOptionFailed", { error: String(e) }));
    }
  };

  const enabledCount = rules.filter((r) => r.enabled).length;

  return (
    <div style={{ padding: "8px", height: "100%", display: "flex", flexDirection: "column", gap: "6px", fontSize: "13px" }}>
      {/* Toolbar */}
      <div style={{ display: "flex", gap: "4px", alignItems: "center", flexWrap: "wrap" }}>
        <div style={{ display: "flex", gap: "4px", flex: 1, minWidth: "200px" }}>
          <input
            className="ui-input"
            style={{ flex: 1, fontSize: "12px", padding: "2px 6px" }}
            type="text"
            value={filePath}
            onChange={(e) => setFilePath(e.target.value)}
            placeholder={t("headerProcessor.rulesPathPlaceholder")}
          />
          <Button variant="ghost" size="xs" onClick={loadFile} loading={loading}>
            {t("headerProcessor.load")}
          </Button>
          <Button variant="ghost" size="xs" onClick={saveRules} loading={loading} disabled={!filePath}>
            <Save size={12} />
          </Button>
        </div>
        <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
          <Button variant="ghost" size="xs" onClick={addRule} loading={loading} title={t("headerProcessor.addRule")}>
            <Plus size={12} />
          </Button>
          <Button variant="primary" size="xs" onClick={applyRules} loading={loading} disabled={rules.length === 0}>
            {t("headerProcessor.apply", { count: enabledCount })}
          </Button>
        </div>
      </div>

      {/* Templates */}
      <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
        <input
          className="ui-input"
          style={{ flex: 1, fontSize: "11px", padding: "2px 6px" }}
          type="text"
          value={templateDir}
          onChange={(e) => setTemplateDir(e.target.value)}
          placeholder={t("headerProcessor.templatesDirPlaceholder")}
        />
        <Button variant="ghost" size="xs" onClick={refreshTemplates} title={t("headerProcessor.listTemplates")}>
          <FolderOpen size={12} />
        </Button>
        <input
          className="ui-input"
          style={{ width: "100px", fontSize: "11px", padding: "2px 6px" }}
          type="text"
          value={templateName}
          onChange={(e) => setTemplateName(e.target.value)}
          placeholder={t("headerProcessor.templateName")}
        />
        <Button variant="ghost" size="xs" onClick={saveTemplate} disabled={!templateDir || !templateName} title={t("headerProcessor.saveTemplate")}>
          <Save size={12} />
        </Button>
        {templates.length > 0 && (
          <select
            className="ui-input"
            style={{ fontSize: "11px", padding: "2px 4px", maxWidth: "140px" }}
            value=""
            onChange={(e) => {
              if (e.target.value === "__delete__") return;
              if (e.target.value) loadTemplate(e.target.value);
            }}
          >
            <option value="">{t("headerProcessor.loadTemplate")}</option>
            {templates.map((t) => (
              <option key={t.name} value={t.name}>
                {t.name} ({t.enabled_count}/{t.rule_count})
              </option>
            ))}
          </select>
        )}
        {templates.length > 0 && (
          <select
            className="ui-input"
            style={{ fontSize: "11px", padding: "2px 4px", maxWidth: "80px", color: "var(--color-danger)" }}
            value=""
            onChange={(e) => {
              if (e.target.value) deleteTemplate(e.target.value);
            }}
          >
            <option value="">{t("headerProcessor.deleteTemplate")}</option>
            {templates.map((t) => (
              <option key={t.name} value={t.name}>
                {t.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {/* Search */}
      <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
        <Search size={12} style={{ color: "var(--color-muted)", flexShrink: 0 }} />
          <input
            className="ui-input"
            style={{ flex: 1, fontSize: "12px", padding: "2px 6px" }}
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={t("headerProcessor.filterRules")}
          />
      </div>

      {/* Result */}
      {result && (
        <div className="dialog-section" style={{ padding: "2px 8px", fontSize: "12px" }}>
          {t("headerProcessor.matched")}: <strong>{result.strings_matched}</strong> {t("headerProcessor.strings")} ({result.enabled_rules}/{result.total_rules} {t("headerProcessor.rules")})
        </div>
      )}

      {/* Rule list */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {filteredRules.map((rule) => {
          const isExpanded = expanded.has(rule.index);
          const summaryParts = [
            rule.in_edid.length > 0 ? t("headerProcessor.summaryEdid", { count: rule.in_edid.length }) : null,
            rule.include_keywords.length > 0 ? t("headerProcessor.summaryInclude", { count: rule.include_keywords.length }) : null,
            rule.exclude_keywords.length > 0 ? t("headerProcessor.summaryExclude", { count: rule.exclude_keywords.length }) : null,
            rule.regex ? t("headerProcessor.summaryRegex") : null,
            rule.full_replace ? t("headerProcessor.summaryFull") : null,
          ].filter((part): part is string => Boolean(part));

          return (
            <div
              key={rule.index}
              style={{
                borderBottom: "1px solid var(--color-border)",
                padding: "2px 4px",
                opacity: rule.enabled ? 1 : 0.4,
              }}
            >
              {/* Rule row */}
              <div
                style={{ display: "flex", alignItems: "center", gap: "4px", cursor: "pointer", minHeight: "24px" }}
                onClick={() => toggleExpanded(rule.index)}
              >
                <input
                  type="checkbox"
                  checked={rule.enabled}
                  onChange={(e) => {
                    e.stopPropagation();
                    toggleRule(rule.index, e.target.checked);
                  }}
                  style={{ flexShrink: 0 }}
                />
                {isExpanded ? (
                  <div style={{ display: "flex", gap: "2px", flexWrap: "wrap", flex: 1 }}>
                    {editing?.index === rule.index && editing?.field === "r_sig" ? (
                      editableField(rule.r_sig, "rSig", (v) => updateField(rule.index, "r_sig", v.toUpperCase()), { width: "42px" })
                    ) : (
                      <span
                        style={{ color: "var(--color-accent)", fontWeight: 600, cursor: "pointer" }}
                        onClick={(e) => { e.stopPropagation(); setEditing({ index: rule.index, field: "r_sig" }); }}
                        title={t("headerProcessor.clickToEdit")}
                      >
                        {rule.r_sig || "?"}
                      </span>
                    )}
                    <span style={{ color: "var(--color-muted)" }}>:</span>
                    {editing?.index === rule.index && editing?.field === "f_sig" ? (
                      editableField(rule.f_sig, "fSig", (v) => updateField(rule.index, "f_sig", v.toUpperCase()), { width: "42px" })
                    ) : (
                      <span
                        style={{ color: "var(--color-accent)", fontWeight: 600, cursor: "pointer" }}
                        onClick={(e) => { e.stopPropagation(); setEditing({ index: rule.index, field: "f_sig" }); }}
                        title={t("headerProcessor.clickToEdit")}
                      >
                        {rule.f_sig || "?"}
                      </span>
                    )}
                    {editing?.index === rule.index && editing?.field === "header" ? (
                      editableField(rule.header, t("headerProcessor.header"), (v) => updateField(rule.index, "header", v), { minWidth: "80px" })
                    ) : (
                      <span
                        style={{ marginLeft: "4px", cursor: "pointer" }}
                        onClick={(e) => { e.stopPropagation(); setEditing({ index: rule.index, field: "header" }); }}
                        title={t("headerProcessor.clickToEdit")}
                      >
                        {rule.header || <span style={{ color: "var(--color-muted)", fontStyle: "italic" }}>{t("headerProcessor.noHeader")}</span>}
                      </span>
                    )}
                  </div>
                ) : (
                  <span style={{ flex: 1, display: "flex", gap: "4px", overflow: "hidden" }}>
                    <span style={{ color: "var(--color-accent)", minWidth: "48px", flexShrink: 0 }}>
                      {rule.r_sig}:{rule.f_sig}
                    </span>
                    <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {rule.header || t("headerProcessor.noHeader")}
                    </span>
                  </span>
                )}
                <span style={{ color: "var(--color-muted)", fontSize: "10px", flexShrink: 0 }}>
                  {summaryParts.join(" ")}
                </span>
                {/* Action buttons */}
                <div style={{ display: "flex", gap: "1px", flexShrink: 0 }} onClick={(e) => e.stopPropagation()}>
                    <button
                    className="ui-btn ui-btn-xs ui-btn-ghost"
                    style={{ padding: "0 2px" }}
                    onClick={() => moveRule(rule.index, "up")}
                    disabled={rule.index === 0}
                    title={t("headerProcessor.moveUp")}
                  >
                    <ChevronUp size={11} />
                  </button>
                  <button
                    className="ui-btn ui-btn-xs ui-btn-ghost"
                    style={{ padding: "0 2px" }}
                    onClick={() => moveRule(rule.index, "down")}
                    disabled={rule.index === rules.length - 1}
                    title={t("headerProcessor.moveDown")}
                  >
                    <ChevronDown size={11} />
                  </button>
                  <button
                    className="ui-btn ui-btn-xs ui-btn-ghost"
                    style={{ padding: "0 2px", color: "var(--color-danger)" }}
                    onClick={() => deleteRule(rule.index)}
                    title={t("headerProcessor.deleteRule")}
                  >
                    <Trash2 size={11} />
                  </button>
                </div>
              </div>

              {/* Expanded detail */}
              {isExpanded && (
                <div
                  style={{
                    marginTop: "4px",
                    marginLeft: "20px",
                    padding: "4px 8px",
                    background: "var(--color-surface-raised)",
                    borderRadius: "4px",
                    display: "flex",
                    flexDirection: "column",
                    gap: "3px",
                    fontSize: "12px",
                  }}
                  onClick={(e) => e.stopPropagation()}
                >
                  {/* Editable fields */}
                  <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
                    <label style={{ width: "55px", color: "var(--color-muted)", flexShrink: 0 }}>rSig</label>
                    {editing?.index === rule.index && editing?.field === "r_sig" ? (
                      editableField(rule.r_sig, "rSig", (v) => updateField(rule.index, "r_sig", v.toUpperCase()), { width: "60px" })
                    ) : (
                      <span style={{ width: "60px", cursor: "pointer" }} onClick={() => setEditing({ index: rule.index, field: "r_sig" })}>
                        {rule.r_sig || "—"}
                      </span>
                    )}
                    <label style={{ width: "55px", color: "var(--color-muted)", flexShrink: 0 }}>fSig</label>
                    {editing?.index === rule.index && editing?.field === "f_sig" ? (
                      editableField(rule.f_sig, "fSig", (v) => updateField(rule.index, "f_sig", v.toUpperCase()), { width: "60px" })
                    ) : (
                      <span style={{ width: "60px", cursor: "pointer" }} onClick={() => setEditing({ index: rule.index, field: "f_sig" })}>
                        {rule.f_sig || "—"}
                      </span>
                    )}
                    <label style={{ color: "var(--color-muted)", flexShrink: 0, marginLeft: "8px" }}>{t("headerProcessor.full")}</label>
                    <input
                      type="checkbox"
                      checked={rule.full_replace}
                      onChange={(e) => updateField(rule.index, "full_replace", e.target.checked ? "true" : "false")}
                    />
                  </div>

                  <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
                    <label style={{ width: "55px", color: "var(--color-muted)", flexShrink: 0 }}>{t("headerProcessor.header")}</label>
                    {editing?.index === rule.index && editing?.field === "header" ? (
                      editableField(rule.header, t("headerProcessor.headerText"), (v) => updateField(rule.index, "header", v), { flex: 1 })
                    ) : (
                      <span style={{ flex: 1, cursor: "pointer" }} onClick={() => setEditing({ index: rule.index, field: "header" })}>
                        {rule.header || <span style={{ color: "var(--color-muted)", fontStyle: "italic" }}>{t("headerProcessor.empty")}</span>}
                      </span>
                    )}
                  </div>

                  <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
                    <label style={{ width: "55px", color: "var(--color-muted)", flexShrink: 0 }}>inEDID</label>
                    {editing?.index === rule.index && editing?.field === "in_edid" ? (
                      editableField(rule.in_edid.join("|"), t("headerProcessor.patternsPlaceholder"), (v) => updateField(rule.index, "in_edid", v), { flex: 1 })
                    ) : (
                      <span style={{ flex: 1, cursor: "pointer", color: rule.in_edid.length ? "inherit" : "var(--color-muted)", fontStyle: rule.in_edid.length ? "inherit" : "italic" }}
                        onClick={() => setEditing({ index: rule.index, field: "in_edid" })}>
                        {rule.in_edid.length ? rule.in_edid.join(" | ") : t("headerProcessor.none")}
                      </span>
                    )}
                  </div>

                  <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
                    <label style={{ width: "55px", color: "var(--color-muted)", flexShrink: 0 }}>exEDID</label>
                    {editing?.index === rule.index && editing?.field === "ex_edid" ? (
                      editableField(rule.ex_edid.join("|"), t("headerProcessor.excludePatternsPlaceholder"), (v) => updateField(rule.index, "ex_edid", v), { flex: 1 })
                    ) : (
                      <span style={{ flex: 1, cursor: "pointer", color: rule.ex_edid.length ? "inherit" : "var(--color-muted)", fontStyle: rule.ex_edid.length ? "inherit" : "italic" }}
                        onClick={() => setEditing({ index: rule.index, field: "ex_edid" })}>
                        {rule.ex_edid.length ? rule.ex_edid.join(" | ") : t("headerProcessor.none")}
                      </span>
                    )}
                  </div>

                  <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
                    <label style={{ width: "55px", color: "var(--color-muted)", flexShrink: 0 }}>{t("headerProcessor.regex")}</label>
                    {editing?.index === rule.index && editing?.field === "regex" ? (
                      editableField(rule.regex || "", t("headerProcessor.regexPlaceholder"), (v) => updateField(rule.index, "regex", v), { flex: 1 })
                    ) : (
                      <span style={{ flex: 1, cursor: "pointer", color: rule.regex ? "inherit" : "var(--color-muted)", fontStyle: rule.regex ? "inherit" : "italic" }}
                        onClick={() => setEditing({ index: rule.index, field: "regex" })}>
                        {rule.regex || t("headerProcessor.none")}
                      </span>
                    )}
                  </div>

                  {rule.include_keywords.length > 0 && (
                    <div style={{ fontSize: "11px", color: "var(--color-muted)" }}>
                      {t("headerProcessor.include")}:{rule.include_keywords.map((k) => `${k.kw_type}:${k.name}`).join(", ")}
                    </div>
                  )}
                  {rule.exclude_keywords.length > 0 && (
                    <div style={{ fontSize: "11px", color: "var(--color-muted)" }}>
                      {t("headerProcessor.exclude")}:{rule.exclude_keywords.map((k) => `${k.kw_type}:${k.name}`).join(", ")}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
        {filteredRules.length === 0 && (
          <div style={{ padding: "16px", textAlign: "center", color: "var(--color-muted)", fontSize: "13px" }}>
            {rules.length === 0 ? t("headerProcessor.noRulesLoaded") : t("headerProcessor.noRulesMatch")}
          </div>
        )}
      </div>

      {/* Pre-processing Options */}
      <details style={{ fontSize: "12px" }}>
        <summary style={{ cursor: "pointer", padding: "2px 0", color: "var(--color-muted)" }}>
          {t("headerProcessor.optionsTitle", { count: opts.length })}
        </summary>
        <div style={{ display: "flex", gap: "4px", marginTop: "4px", alignItems: "center" }}>
          <input className="ui-input" style={{ flex: 1, fontSize: "11px", padding: "2px 6px" }} type="text"
            value={optsPath} onChange={(e) => setOptsPath(e.target.value)}
            placeholder={t("headerProcessor.optionsPathPlaceholder")} />
          <Button variant="ghost" size="xs" onClick={loadOpts}>{t("headerProcessor.load")}</Button>
          <Button variant="ghost" size="xs" onClick={saveOpts}><Save size={11} /></Button>
        </div>
        <div style={{ display: "flex", gap: "4px", marginTop: "4px" }}>
          <input className="ui-input" style={{ width: "100px", fontSize: "11px", padding: "2px 4px" }} type="text"
            value={newOptKey} onChange={(e) => setNewOptKey(e.target.value)} placeholder={t("headerProcessor.key")} />
          <input className="ui-input" style={{ flex: 1, fontSize: "11px", padding: "2px 4px" }} type="text"
            value={newOptVal} onChange={(e) => setNewOptVal(e.target.value)} placeholder={t("headerProcessor.value")} />
          <Button variant="ghost" size="xs" onClick={addOpt} disabled={!newOptKey}><Plus size={11} /></Button>
        </div>
        <div style={{ maxHeight: "120px", overflow: "auto", marginTop: "4px" }}>
          {opts.map(([k, v]) => (
            <div key={k} style={{ display: "flex", alignItems: "center", gap: "4px", padding: "1px 4px", borderBottom: "1px solid var(--color-border)" }}>
              <span style={{ color: "var(--color-accent)", minWidth: "80px", fontSize: "11px" }}>{k}</span>
              <span style={{ flex: 1, fontSize: "11px" }}>{v}</span>
              <button className="ui-btn ui-btn-xs ui-btn-ghost" style={{ padding: "0 2px", color: "var(--color-danger)" }}
                onClick={() => deleteOpt(k)}><Trash2 size={10} /></button>
            </div>
          ))}
        </div>
      </details>
    </div>
  );
}
