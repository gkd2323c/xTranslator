import { useMemo } from "react";
import { useAppStore } from "../stores/appStore";
import { FileText, Languages, Database, BarChart3, Code2 } from "lucide-react";
import { useTranslation } from "react-i18next";

export function SidePanel() {
  const { t } = useTranslation();
  const espPath = useAppStore((s) => s.espPath);
  const espStats = useAppStore((s) => s.espStats);
  const sstStats = useAppStore((s) => s.sstStats);
  const allItems = useAppStore((s) => s.allItems);
  const recordFilter = useAppStore((s) => s.recordFilter);
  const loadProgress = useAppStore((s) => s.loadProgress);
  const setRecordFilter = useAppStore((s) => s.setRecordFilter);
  const vmadFilter = useAppStore((s) => s.vmadFilter);
  const setVmadFilter = useAppStore((s) => s.setVmadFilter);
  const espMode = useAppStore((s) => s.espMode);

  const { translated, incomplete, locked, vmadCount } = useMemo(() => {
    let t = 0, inc = 0, vmad = 0;
    for (const item of allItems) {
      if (item.status === "translated") t++;
      else if (item.status === "incomplete") inc++;
      if (item.is_vmad) vmad++;
    }
    return { translated: t, incomplete: inc, locked: allItems.length - t - inc, vmadCount: vmad };
  }, [allItems]);

  // 如果正在加载，显示加载进度
  if (loadProgress) {
    const isXmlOp = loadProgress.stage === "parsing" || loadProgress.stage === "merging" || loadProgress.stage === "writing" || loadProgress.stage === "collecting";
    const title = isXmlOp ? "Processing XML..." : "Loading ESP...";
    return (
      <div className="sidepanel">
        <div className="sidepanel-section" style={{ paddingTop: 16 }}>
          <h3 style={{ marginBottom: 12 }}>{title}</h3>
          <div style={{ marginBottom: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6, fontSize: 12 }}>
              <span style={{ color: "var(--text-secondary)" }}>{loadProgress.stage}</span>
              <span style={{ color: "var(--accent-cyan)", fontFamily: "'JetBrains Mono', monospace", fontWeight: 600 }}>
                {loadProgress.percentage}%
              </span>
            </div>
            <div style={{
              width: "100%",
              height: 8,
              backgroundColor: "var(--bg-secondary)",
              borderRadius: 4,
              overflow: "hidden",
            }}>
              <div style={{
                width: `${loadProgress.percentage}%`,
                height: "100%",
                backgroundColor: "var(--accent-cyan)",
                transition: "width 0.2s ease-out",
              }} />
            </div>
          </div>
          <p style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 8 }}>
            {loadProgress.message}
          </p>
        </div>
      </div>
    );
  }

  if (!espStats) {
    return (
      <div className="sidepanel">
        <div className="sidepanel-empty">
          <FileText size={48} opacity={0.3} />
          <p>{t("sidebar.noEspLoaded")}</p>
          <p className="sidepanel-hint">{t("sidebar.loadEspToStart")}</p>
        </div>
      </div>
    );
  }

  const totalTranslatable = translated + incomplete;
  const progressPercent = totalTranslatable > 0 ? (translated / totalTranslatable) * 100 : 0;

  return (
    <div className="sidepanel">
      <div className="sidepanel-section">
        <h3><FileText size={16} /> {t("sidebar.fileInfo")}</h3>
        <div className="sidepanel-row">
          <span className="sidepanel-label">ESP</span>
          <span className="sidepanel-value file-path" title={espPath || ""}>
            {espPath?.split(/[\\/]/).pop() || "\u2014"}
          </span>
        </div>
        <div className="sidepanel-row">
          <span className="sidepanel-label">{t("sidebar.strings")}</span>
          <span className="sidepanel-value">{espStats.strings_loaded} {t("sidebar.filesLoaded")}</span>
        </div>
        <div className="sidepanel-row">
          <span className="sidepanel-label">{t("sidebar.parseTime")}</span>
          <span className="sidepanel-value">{espStats.parse_time_ms}ms</span>
        </div>
        <div className="sidepanel-row">
          <span className="sidepanel-label">{t("sidebar.saveMode", { defaultValue: "Save Mode" })}</span>
          <span className="sidepanel-value" style={{ color: espMode ? "#4caf50" : undefined }}>
            {espMode ? t("sidebar.espMode", { defaultValue: "ESP mode" }) : t("sidebar.stringsMode", { defaultValue: "Strings mode" })}
          </span>
        </div>
      </div>

      <div className="sidepanel-section">
        <h3><Database size={16} /> Statistics</h3>
        <div className="sidepanel-row">
          <span className="sidepanel-label">{t("sidebar.totalStrings")}</span>
          <span className="sidepanel-value">{espStats.total.toLocaleString()}</span>
        </div>
        <div className="sidepanel-row">
          <span className="sidepanel-label status-translated">{"\u25CF"} {t("sidebar.translatedCount")}</span>
          <span className="sidepanel-value">{translated.toLocaleString()}</span>
        </div>
        <div className="sidepanel-row">
          <span className="sidepanel-label status-incomplete">{"\u25CF"} {t("sidebar.incompleteCount")}</span>
          <span className="sidepanel-value">{incomplete.toLocaleString()}</span>
        </div>
        <div className="sidepanel-row">
          <span className="sidepanel-label status-locked">{"\u25CF"} {t("sidebar.lockedCount")}</span>
          <span className="sidepanel-value">{locked.toLocaleString()}</span>
        </div>
        <div className="sidepanel-row">
          <span className="sidepanel-label" style={{ color: "var(--accent-purple)" }}>{"\u25CF"} {t("sidebar.vmad", { defaultValue: "VMAD" })}</span>
          <span className="sidepanel-value">{vmadCount.toLocaleString()}</span>
        </div>
        <div style={{ marginTop: 12, paddingTop: 12, borderTop: "1px solid var(--border-subtle)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6, fontSize: 11 }}>
            <span style={{ color: "var(--text-secondary)" }}>{t("sidebar.progress")}</span>
            <span style={{ color: "var(--accent-cyan)", fontFamily: "'JetBrains Mono', monospace", fontWeight: 600 }}>
              {progressPercent.toFixed(1)}%
            </span>
          </div>
          <div style={{ height: 4, background: "var(--bg-primary)", borderRadius: 2, overflow: "hidden", position: "relative" }}>
            <div style={{
              height: "100%",
              width: `${progressPercent}%`,
              background: "linear-gradient(90deg, var(--accent-cyan), var(--success))",
              borderRadius: 2,
              boxShadow: "0 0 10px var(--accent-cyan)",
              transition: "width 0.5s ease"
            }} />
          </div>
        </div>
      </div>

      {sstStats && (
        <div className="sidepanel-section">
          <h3><Languages size={16} /> SST</h3>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.matched")}</span>
            <span className="sidepanel-value status-translated">{sstStats.matched}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.unmatched")}</span>
            <span className="sidepanel-value status-incomplete">{sstStats.unmatched}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.exact")}</span>
            <span className="sidepanel-value">{sstStats.tier_exact}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.edid")}</span>
            <span className="sidepanel-value">{sstStats.tier_edid}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.normalized")}</span>
            <span className="sidepanel-value">{sstStats.tier_normalized}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.vocab")}</span>
            <span className="sidepanel-value">{sstStats.tier_vocab}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.ambiguous")}</span>
            <span className="sidepanel-value">{sstStats.ambiguous}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.pending")}</span>
            <span className="sidepanel-value">{sstStats.pending_skipped}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.oldData")}</span>
            <span className="sidepanel-value">{sstStats.old_data_preserved}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.warnings")}</span>
            <span className="sidepanel-value">{sstStats.warning}/{sstStats.big_warning}</span>
          </div>
          <div className="sidepanel-row">
            <span className="sidepanel-label">{t("sidebar.updated")}</span>
            <span className="sidepanel-value">{sstStats.updated_ids.length}</span>
          </div>
        </div>
      )}

      {vmadCount > 0 && (
        <div className="sidepanel-section">
          <h3><Code2 size={16} /> {t("vmad.title")}</h3>
          <div
            className={`sidepanel-row record-type-row ${vmadFilter ? "active" : ""}`}
            onClick={() => setVmadFilter(!vmadFilter)}
            title={vmadFilter ? t("vmad.showAll") : t("vmad.showOnly")}
            style={{ cursor: "pointer" }}
          >
            <span className="sidepanel-label" style={{ color: vmadFilter ? "var(--accent-purple)" : "inherit" }}>
              {vmadFilter ? t("vmad.showingVmadOnly") : t("vmad.filterVmAD")}
            </span>
            <span className="sidepanel-value">{vmadCount.toLocaleString()}</span>
          </div>
          {vmadFilter && (
            <div style={{ marginTop: 8, textAlign: "center" }}>
              <button
                onClick={() => setVmadFilter(false)}
                className="btn btn-ghost btn-sm"
                style={{ fontSize: 11, padding: "4px 12px" }}
              >
                Clear filter
              </button>
            </div>
          )}
        </div>
      )}

      {espStats.record_counts && Object.keys(espStats.record_counts).length > 0 && (
        <div className="sidepanel-section">
          <h3><BarChart3 size={16} /> {t("sidebar.recordTypes")}</h3>
          {Object.entries(espStats.record_counts)
            .sort((a, b) => b[1] - a[1])
            .slice(0, 10)
            .map(([sig, count]) => {
              const isActive = recordFilter === sig;
              return (
                <div
                  key={sig}
                  className={`sidepanel-row record-type-row ${isActive ? "active" : ""}`}
                  onClick={() => setRecordFilter(isActive ? null : sig)}
                  title={isActive ? t("sidebar.clearFilterTooltip") : t("sidebar.filterByType")}
                >
                  <span className="sidepanel-label record-sig">{sig}</span>
                  <span className="sidepanel-value">{count.toLocaleString()}</span>
                </div>
              );
            })}
          {recordFilter && (
            <div style={{ marginTop: 8, textAlign: "center" }}>
              <button
                onClick={() => setRecordFilter(null)}
                className="btn btn-ghost btn-sm"
                style={{ fontSize: 11, padding: "4px 12px" }}
              >
                Clear filter
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
