import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { EmptyState } from "../ui";

export function HeuristicPanel() {
  const { t } = useTranslation();

  return (
    <div className="bottom-panel-inner">
      <EmptyState
        icon={<Search size={32} />}
        title={t("bottomTabs.heuristic")}
        hint={t("heuristicPanel.hint")}
      />
    </div>
  );
}
