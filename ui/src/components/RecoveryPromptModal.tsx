import { useAppStore } from "../stores/appStore";
import { RefreshCw, Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";

export function RecoveryPromptModal() {
  const { t } = useTranslation();
  const showRecoveryModal = useAppStore((s) => s.showRecoveryModal);
  const recoveryInfo = useAppStore((s) => s.recoveryInfo);
  const applyRecovery = useAppStore((s) => s.applyRecovery);
  const discardRecovery = useAppStore((s) => s.discardRecovery);
  const closeRecoveryModal = useAppStore((s) => s.closeRecoveryModal);

  if (!showRecoveryModal || !recoveryInfo) return null;

  return (
    <div className="modal-overlay" onClick={closeRecoveryModal}>
      <div className="modal-content recovery-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>
            <RefreshCw size={18} />
            {t("recovery.title", "Unapplied Translations Found")}
          </h3>
          <button className="btn btn-ghost btn-sm" onClick={closeRecoveryModal}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <p>
            {t("recovery.description", {
              count: recoveryInfo.pending_count,
              esp: recoveryInfo.esp_name,
              defaultValue: `Found {{count}} unapplied translations from a previous session for "{{esp}}".`,
            })}
          </p>
          <p className="recovery-hint">
            {t("recovery.hint", "These translations were saved to a crash-safe cache but not yet applied to the ESP data.")}
          </p>
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={discardRecovery}>
            <Trash2 size={14} />
            {t("recovery.discard", "Discard")}
          </button>
          <button className="btn btn-primary" onClick={applyRecovery}>
            <RefreshCw size={14} />
            {t("recovery.apply", "Recover")}
          </button>
        </div>
      </div>
    </div>
  );
}
