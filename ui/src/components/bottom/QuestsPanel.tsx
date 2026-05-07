import { useTranslation } from "react-i18next";
import { MessageSquare } from "lucide-react";
import { EmptyState } from "../ui";

export function QuestsPanel() {
  const { t } = useTranslation();

  return (
    <div className="bottom-panel-inner">
      <EmptyState
        icon={<MessageSquare size={32} />}
        title={t("bottomTabs.quests")}
        hint={t("questsPanel.hint")}
      />
    </div>
  );
}
