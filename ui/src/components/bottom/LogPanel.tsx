import { useTranslation } from "react-i18next";
import { ScrollText } from "lucide-react";
import { EmptyState } from "../ui";

export function LogPanel() {
  const { t } = useTranslation();

  return (
    <div className="bottom-panel-inner">
      <EmptyState
        icon={<ScrollText size={32} />}
        title={t("bottomTabs.log")}
        hint={t("logPanel.hint")}
      />
    </div>
  );
}
