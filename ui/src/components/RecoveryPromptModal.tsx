import { useAppStore } from "../stores/appStore";
import { RefreshCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button, Modal } from "./ui";

export function RecoveryPromptModal() {
  const { t } = useTranslation();
  const showRecoveryModal = useAppStore((s) => s.showRecoveryModal);
  const recoveryInfo = useAppStore((s) => s.recoveryInfo);
  const applyRecovery = useAppStore((s) => s.applyRecovery);
  const discardRecovery = useAppStore((s) => s.discardRecovery);
  const closeRecoveryModal = useAppStore((s) => s.closeRecoveryModal);

  return (
    <Modal
      open={showRecoveryModal && !!recoveryInfo}
      onClose={closeRecoveryModal}
      title={t("recovery.title", "Unapplied Translations Found")}
      size="sm"
      footer={
        <>
          <Button variant="default" onClick={discardRecovery} icon={<Trash2 size={14} />}>
            {t("recovery.discard", "Discard")}
          </Button>
          <Button variant="primary" onClick={applyRecovery} icon={<RefreshCw size={14} />}>
            {t("recovery.apply", "Recover")}
          </Button>
        </>
      }
    >
      {recoveryInfo && (
        <>
          <p>
            {t("recovery.description", {
              count: recoveryInfo.pending_count,
              esp: recoveryInfo.esp_name,
              defaultValue: `Found {{count}} unapplied translations from a previous session for "{{esp}}".`,
            })}
          </p>
          <p className="ui-modal-hint">
            {t("recovery.hint", "These translations were saved to a crash-safe cache but not yet applied to the ESP data.")}
          </p>
        </>
      )}
    </Modal>
  );
}
