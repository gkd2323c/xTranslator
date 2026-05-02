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
      .catch((e) => toast.error(`Failed to load ESP header: ${e}`))
      .finally(() => setLoading(false));
  }, [espPath]);

  if (!espPath) {
    return (
      <div className="bottom-panel-inner">
        <EmptyState
          icon={<TreePine size={32} />}
          title={t("bottomTabs.espTree", { defaultValue: "ESP Tree" })}
          hint="Load an ESP/ESM file to view its header information"
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
              <span className="esp-tree-label">Version:</span>
              <span className="esp-tree-value">{header.version}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">Records:</span>
              <span className="esp-tree-value">{header.num_records.toLocaleString()}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">Masters:</span>
              <span className="esp-tree-value">{header.masters?.join(", ") || "—"}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">Author:</span>
              <span className="esp-tree-value">{header.author || "—"}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">Description:</span>
              <span className="esp-tree-value">{header.description || "—"}</span>
            </div>
            <div className="esp-tree-row">
              <span className="esp-tree-label">Flags:</span>
              <span className="esp-tree-value">
                {header.is_master ? "Master" : "Plugin"}
                {header.is_localized ? " · Localized" : ""}
              </span>
            </div>
          </div>
          <p className="esp-tree-hint">Full GRUP tree view coming soon</p>
        </div>
      ) : (
        <EmptyState
          icon={<TreePine size={32} />}
          title="No ESP data"
          hint="Failed to load ESP header information"
        />
      )}
    </div>
  );
}
