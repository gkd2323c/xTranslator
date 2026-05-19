import { useMemo, useState, useEffect } from "react";
import { useAppStore } from "../stores/appStore";
import { FileText, Languages, Database, BarChart3, Code2, Info } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Section, KeyValueRow, ProgressBar, EmptyState, Button } from "./ui";
import { getEspHeader } from "../api/strings";
import type { EspHeaderInfoDto } from "../api/strings";

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

  const [headerInfo, setHeaderInfo] = useState<EspHeaderInfoDto | null>(null);

  useEffect(() => {
    if (espMode && espPath) {
      getEspHeader()
        .then(setHeaderInfo)
        .catch(() => setHeaderInfo(null));
    } else {
      setHeaderInfo(null);
    }
  }, [espMode, espPath]);

  const { translated, incomplete, locked, vmadCount } = useMemo(() => {
    let t = 0, inc = 0, vmad = 0;
    for (const item of allItems) {
      if (item.status === "translated") t++;
      else if (item.status === "incomplete") inc++;
      if (item.is_vmad) vmad++;
    }
    return { translated: t, incomplete: inc, locked: allItems.length - t - inc, vmadCount: vmad };
  }, [allItems]);

  const totalTranslatable = translated + incomplete;
  const progressPercent = totalTranslatable > 0 ? (translated / totalTranslatable) * 100 : 0;
  const yesLabel = t("common.yes");
  const noLabel = t("common.no");

  const statusContent = !espStats ? (
    <Section icon={<FileText size={16} />} title={t("sidebar.fileInfo")}>
      <EmptyState
        icon={<FileText size={36} />}
        title={t("sidebar.noEspLoaded")}
        hint={t("sidebar.loadEspToStart")}
      />
    </Section>
  ) : (
    <>
      <Section icon={<FileText size={16} />} title={t("sidebar.fileInfo")}>
        <KeyValueRow label="ESP" value={espPath?.split(/[\\/]/).pop() || "—"} />
        <KeyValueRow label={t("sidebar.strings")} value={`${espStats.strings_loaded} ${t("sidebar.filesLoaded")}`} />
        <KeyValueRow label={t("sidebar.parseTime")} value={`${espStats.parse_time_ms}ms`} />
        <KeyValueRow
          label={t("sidebar.saveMode")}
          value={espMode ? t("sidebar.espMode") : t("sidebar.stringsMode")}
          valueClassName={espMode ? "ui-status-translated" : ""}
        />
        <KeyValueRow
          label={t("sidebar.localization")}
          value={
            espStats.strings_loaded > 0
              ? t("sidebar.delocalized")
              : t("sidebar.localized")
          }
          valueClassName={espStats.strings_loaded > 0 ? "" : "ui-status-incomplete"}
        />
      </Section>

      {espMode && headerInfo && (
        <Section icon={<Info size={16} />} title={t("sidebar.espHeader")}>
          <KeyValueRow label={t("sidebar.author")} value={headerInfo.author || "—"} />
          <KeyValueRow label={t("sidebar.description")} value={headerInfo.description || "—"} />
          <KeyValueRow label={t("sidebar.version")} value={headerInfo.version.toFixed(2)} />
          <KeyValueRow label={t("sidebar.records")} value={headerInfo.num_records.toLocaleString()} />
          <KeyValueRow label={t("sidebar.nextObjectId")} value={`0x${headerInfo.next_object_id.toString(16).toUpperCase()}`} />
          <KeyValueRow label={t("sidebar.masterFile")} value={headerInfo.is_master ? yesLabel : noLabel} />
          <KeyValueRow label={t("sidebar.localized")} value={headerInfo.is_localized ? yesLabel : noLabel} />
          {headerInfo.masters.length > 0 && (
            <KeyValueRow
              label={t("sidebar.masterFiles")}
              value={headerInfo.masters.join(", ")}
            />
          )}
        </Section>
      )}
      <Section icon={<Database size={16} />} title={t("sidebar.statistics")}>
        {/* 状态概览卡片 */}
        <div className="stats-cards">
          <div className="stats-card">
            <span className="stats-card-value">{espStats.total.toLocaleString()}</span>
            <span className="stats-card-label">{t("sidebar.totalStrings")}</span>
          </div>
          <div className="stats-card stats-card-translated">
            <span className="stats-card-value">{translated.toLocaleString()}</span>
            <span className="stats-card-label">{t("sidebar.translatedCount")}</span>
          </div>
          <div className="stats-card stats-card-incomplete">
            <span className="stats-card-value">{incomplete.toLocaleString()}</span>
            <span className="stats-card-label">{t("sidebar.incompleteCount")}</span>
          </div>
          <div className="stats-card stats-card-locked">
            <span className="stats-card-value">{locked.toLocaleString()}</span>
            <span className="stats-card-label">{t("sidebar.lockedCount")}</span>
          </div>
        </div>

        {/* 大号进度条 */}
        {/* 大号进度条 */}
        <div className="stats-progress-block">
          <div className="stats-progress-header">
            <span className="stats-progress-pct">{progressPercent.toFixed(1)}%</span>
            <span className="stats-progress-detail">{translated.toLocaleString()} / {totalTranslatable.toLocaleString()} {t("sidebar.translatable", { defaultValue: "translatable" })}</span>
          </div>
          <div className="stats-progress-bg">
            <div className="stats-progress-fill" style={{ width: `${progressPercent}%` }} />
          </div>
        </div>

        <KeyValueRow label={t("sidebar.vmad")} value={vmadCount.toLocaleString()} labelClassName="ui-status-script" />
      </Section>
      {sstStats && (
        <Section icon={<Languages size={16} />} title={t("sidebar.sst")}>
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
                {t("common.clearFilter")}
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
                {t("common.clearFilter")}
              </Button>
            </div>
          )}
        </Section>
      )}
    </>
  );

  if (loadProgress) {
    const isXmlOp = loadProgress.stage === "parsing" || loadProgress.stage === "merging" || loadProgress.stage === "writing" || loadProgress.stage === "collecting";
    const title = isXmlOp ? t("sidebar.processingXml") : t("sidebar.loadingEsp");
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

  return (
    <div className="sidepanel">
      <div className="sidepanel-status-block">
        {statusContent}
      </div>
    </div>
  );
}
