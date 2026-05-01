import { useMemo } from "react";
import { useAppStore } from "../stores/appStore";
import { FileText, Languages, Database, BarChart3, Code2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Section, KeyValueRow, ProgressBar, EmptyState, Button } from "./ui";

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

  if (loadProgress) {
    const isXmlOp = loadProgress.stage === "parsing" || loadProgress.stage === "merging" || loadProgress.stage === "writing" || loadProgress.stage === "collecting";
    const title = isXmlOp ? "Processing XML..." : "Loading ESP...";
    return (
      <div className="sidepanel">
        <Section title={title}>
          <ProgressBar
            value={loadProgress.percentage}
            max={100}
            variant="gradient"
            showLabel
            label={loadProgress.stage}
          />
          <p style={{ fontSize: 12, color: "var(--color-muted)", marginTop: "var(--space-xs)" }}>
            {loadProgress.message}
          </p>
        </Section>
      </div>
    );
  }

  if (!espStats) {
    return (
      <div className="sidepanel">
        <EmptyState
          icon={<FileText size={48} />}
          title={t("sidebar.noEspLoaded")}
          hint={t("sidebar.loadEspToStart")}
        />
      </div>
    );
  }

  const totalTranslatable = translated + incomplete;
  const progressPercent = totalTranslatable > 0 ? (translated / totalTranslatable) * 100 : 0;

  return (
    <div className="sidepanel">
      <Section icon={<FileText size={16} />} title={t("sidebar.fileInfo")}>
        <KeyValueRow label="ESP" value={espPath?.split(/[\\/]/).pop() || "—"} />
        <KeyValueRow label={t("sidebar.strings")} value={`${espStats.strings_loaded} ${t("sidebar.filesLoaded")}`} />
        <KeyValueRow label={t("sidebar.parseTime")} value={`${espStats.parse_time_ms}ms`} />
        <KeyValueRow
          label={t("sidebar.saveMode", { defaultValue: "Save Mode" })}
          value={espMode ? t("sidebar.espMode", { defaultValue: "ESP mode" }) : t("sidebar.stringsMode", { defaultValue: "Strings mode" })}
          valueClassName={espMode ? "ui-status-translated" : ""}
        />
      </Section>

      <Section icon={<Database size={16} />} title="Statistics">
        <KeyValueRow label={t("sidebar.totalStrings")} value={espStats.total.toLocaleString()} />
        <KeyValueRow
          label={t("sidebar.translatedCount")}
          value={translated.toLocaleString()}
          labelClassName="ui-status-translated"
        />
        <KeyValueRow
          label={t("sidebar.incompleteCount")}
          value={incomplete.toLocaleString()}
          labelClassName="ui-status-incomplete"
        />
        <KeyValueRow
          label={t("sidebar.lockedCount")}
          value={locked.toLocaleString()}
          labelClassName="ui-status-locked"
        />
        <KeyValueRow
          label={t("sidebar.vmad", { defaultValue: "VMAD" })}
          value={vmadCount.toLocaleString()}
          labelClassName="ui-status-script"
        />
        <div style={{ marginTop: "var(--space-sm)" }}>
          <ProgressBar
            value={progressPercent}
            max={100}
            variant="gradient"
            size="sm"
            showLabel
            label={t("sidebar.progress")}
          />
        </div>
      </Section>

      {sstStats && (
        <Section icon={<Languages size={16} />} title="SST">
          <KeyValueRow label={t("sidebar.matched")} value={sstStats.matched} valueClassName="ui-status-translated" />
          <KeyValueRow label={t("sidebar.unmatched")} value={sstStats.unmatched} valueClassName="ui-status-incomplete" />
          <KeyValueRow label={t("sidebar.exact")} value={sstStats.tier_exact} />
          <KeyValueRow label={t("sidebar.edid")} value={sstStats.tier_edid} />
          <KeyValueRow label={t("sidebar.normalized")} value={sstStats.tier_normalized} />
          <KeyValueRow label={t("sidebar.vocab")} value={sstStats.tier_vocab} />
          <KeyValueRow label={t("sidebar.ambiguous")} value={sstStats.ambiguous} />
          <KeyValueRow label={t("sidebar.pending")} value={sstStats.pending_skipped} />
          <KeyValueRow label={t("sidebar.oldData")} value={sstStats.old_data_preserved} />
          <KeyValueRow label={t("sidebar.warnings")} value={`${sstStats.warning}/${sstStats.big_warning}`} />
          <KeyValueRow label={t("sidebar.updated")} value={sstStats.updated_ids.length} />
        </Section>
      )}

      {vmadCount > 0 && (
        <Section icon={<Code2 size={16} />} title={t("vmad.title")}>
          <KeyValueRow
            label={vmadFilter ? t("vmad.showingVmadOnly") : t("vmad.filterVmAD")}
            value={vmadCount.toLocaleString()}
            clickable
            onClick={() => setVmadFilter(!vmadFilter)}
            labelClassName={vmadFilter ? "ui-status-script" : ""}
          />
          {vmadFilter && (
            <div style={{ marginTop: "var(--space-xs)", textAlign: "center" }}>
              <Button variant="ghost" size="xs" onClick={() => setVmadFilter(false)}>
                Clear filter
              </Button>
            </div>
          )}
        </Section>
      )}

      {espStats.record_counts && Object.keys(espStats.record_counts).length > 0 && (
        <Section icon={<BarChart3 size={16} />} title={t("sidebar.recordTypes")}>
          {Object.entries(espStats.record_counts)
            .sort((a, b) => b[1] - a[1])
            .slice(0, 10)
            .map(([sig, count]) => {
              const isActive = recordFilter === sig;
              return (
                <KeyValueRow
                  key={sig}
                  label={sig}
                  value={count.toLocaleString()}
                  clickable
                  onClick={() => setRecordFilter(isActive ? null : sig)}
                  labelClassName="mono"
                  className={isActive ? "ui-kv-row-active" : ""}
                />
              );
            })}
          {recordFilter && (
            <div style={{ marginTop: "var(--space-xs)", textAlign: "center" }}>
              <Button variant="ghost" size="xs" onClick={() => setRecordFilter(null)}>
                Clear filter
              </Button>
            </div>
          )}
        </Section>
      )}
    </div>
  );
}
