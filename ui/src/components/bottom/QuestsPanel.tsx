import { useTranslation } from "react-i18next";
import { MessageSquare } from "lucide-react";
import { EmptyState } from "../ui";

export function QuestsPanel() {
  const { t } = useTranslation();

  return (
    <div className="bottom-panel-inner">
      <EmptyState
        icon={<MessageSquare size={32} />}
        title={t("bottomTabs.quests", { defaultValue: "Quest Browser" })}
        hint="Browse quest records and their dialogue chains. Use the Dialogs tab for NPC dialogue trees."
      />
    </div>
  );
}
