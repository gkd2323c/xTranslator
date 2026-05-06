import { useState, useEffect } from "react";
import { headerRulesLoad, headerRulesList, headerRulesToggle, headerRulesApply, type HeaderRuleDto, type HeaderApplyResult } from "../../api/strings";
import toast from "react-hot-toast";
import { Button } from "../ui";

export function HeaderProcessorPanel() {
  const [rules, setRules] = useState<HeaderRuleDto[]>([]);
  const [result, setResult] = useState<HeaderApplyResult | null>(null);
  const [filePath, setFilePath] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    headerRulesList().then(setRules).catch(() => {});
  }, []);

  const loadFile = async () => {
    if (!filePath) return;
    setLoading(true);
    try {
      const loaded = await headerRulesLoad(filePath);
      setRules(loaded);
      toast.success(`Loaded ${loaded.length} rules`);
    } catch (e: any) {
      toast.error(`Load failed: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const toggleRule = async (index: number, enabled: boolean) => {
    try {
      await headerRulesToggle(index, enabled);
      setRules((prev) => prev.map((r) => (r.index === index ? { ...r, enabled } : r)));
    } catch (e: any) {
      toast.error(`Toggle failed: ${e}`);
    }
  };

  const applyRules = async () => {
    setLoading(true);
    try {
      const res = await headerRulesApply();
      setResult(res);
      toast.success(`${res.strings_matched} strings matched (${res.enabled_rules}/${res.total_rules} rules enabled)`);
    } catch (e: any) {
      toast.error(`Apply failed: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const enabledCount = rules.filter((r) => r.enabled).length;

  return (
    <div style={{ padding: "12px", height: "100%", display: "flex", flexDirection: "column", gap: "8px" }}>
      <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
        <input
          className="ui-input"
          style={{ flex: 1 }}
          type="text"
          value={filePath}
          onChange={(e) => setFilePath(e.target.value)}
          placeholder="Path to rules file (e.g. Data/SkyrimSE/HeaderProcessor/defaultRules_Text.txt)"
        />
        <Button variant="ghost" size="sm" onClick={loadFile} loading={loading}>
          Load
        </Button>
        <Button variant="primary" size="sm" onClick={applyRules} loading={loading} disabled={rules.length === 0}>
          Apply ({enabledCount})
        </Button>
      </div>

      {result && (
        <div className="dialog-section" style={{ padding: "4px 8px" }}>
          Matched: <strong>{result.strings_matched}</strong> strings
          (Enabled: {result.enabled_rules}/{result.total_rules} rules)
        </div>
      )}

      <div style={{ flex: 1, overflow: "auto", fontSize: "13px" }}>
        {rules.map((rule) => (
          <label
            key={rule.index}
            className="settings-checkbox-label"
            style={{
              display: "flex",
              padding: "3px 4px",
              borderBottom: "1px solid var(--color-border)",
              cursor: "pointer",
              opacity: rule.enabled ? 1 : 0.5,
            }}
          >
            <input
              type="checkbox"
              checked={rule.enabled}
              onChange={(e) => toggleRule(rule.index, e.target.checked)}
            />
            <span style={{ color: "var(--color-accent)", minWidth: "50px" }}>{rule.r_sig}:{rule.f_sig}</span>
            <span style={{ marginLeft: "8px" }}>{rule.header || "(no header)"}</span>
            <span style={{ marginLeft: "auto", color: "var(--color-muted)", fontSize: "11px" }}>
              {rule.in_edid.length > 0 && `edid:${rule.in_edid.join(",")} `}
              {rule.include_keywords.length > 0 && `kw:${rule.include_keywords.length}`}
            </span>
          </label>
        ))}
      </div>
    </div>
  );
}
