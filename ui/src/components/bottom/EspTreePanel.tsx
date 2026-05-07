import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { TreePine } from "lucide-react";
import toast from "react-hot-toast";
import { getEspHeader } from "../../api/strings";
import type { EspHeaderInfoDto } from "../../api/strings";
import { useAppStore } from "../../stores/appStore";
import { EmptyState, Spinner } from "../ui";

export function EspTreePanel() {
  const { t } = useTranslation();
  const espPath = useAppStore((s) => s.espPath);
  const [header, setHeader] = useState<EspHeaderInfoDto | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!espPath) return;
    setLoading(true);
    getEspHeader()
      .then(setHeader)
      .catch((e) => toast.error(t("espTreePanel.loadFailed", { error: String(e) })))
      .finally(() => setLoading(false));
  }, [espPath, t]);

  if (!espPath) {
    return (
      <div className="bottom-panel-inner">
        <EmptyState
          icon={<TreePine size={32} />}
          title={t("bottomTabs.espTree")}
          hint={t("espTreePanel.emptyHint")}
        />
      </div>
    );
  }

  if (loading) {
    return (
      <div className="bottom-panel-inner bottom-panel-loading">
        <Spinner size={20} />
      </div>
    );
  }

  return (
    <div className="bottom-panel-inner">
      {header ? (
        <div className="esp-tree-content">
          <div className="esp-tree-info">
            <div className="esp-tree-row">
              <span className="esp-tree-label">{t("espTreePanel.version")}:</span>
              <span className="esp-tree-value">{header.version}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">{t("espTreePanel.records")}:</span>
              <span className="esp-tree-value">{header.num_records.toLocaleString()}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">{t("espTreePanel.masters")}:</span>
              <span className="esp-tree-value">{header.masters?.join(", ") || "—"}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">{t("espTreePanel.author")}:</span>
              <span className="esp-tree-value">{header.author || "—"}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">{t("espTreePanel.description")}:</span>
              <span className="esp-tree-value">{header.description || "—"}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">{t("espTreePanel.flags")}:</span>
              <span className="esp-tree-value">
                {header.is_master ? t("espTreePanel.master") : t("espTreePanel.plugin")}
                {header.is_localized ? ` · ${t("espTreePanel.localized")}` : ""}
              </span>
            </div>
          </div>
          <p className="esp-tree-hint">{t("espTreePanel.comingSoon")}</p>
        </div>
      ) : (
        <EmptyState
          icon={<TreePine size={32} />}
          title={t("espTreePanel.noData")}
          hint={t("espTreePanel.loadFailedShort")}
        />
      )}
    </div>
  );
}
