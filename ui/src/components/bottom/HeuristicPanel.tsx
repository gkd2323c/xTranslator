import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { EmptyState } from "../ui";

export function HeuristicPanel() {
  const { t } = useTranslation();

  return (
    <div className="bottom-panel-inner">
      <EmptyState
        icon={<Search size={32} />}
        title={t("bottomTabs.heuristic", { defaultValue: "Heuristic Suggestions" })}
        hint="Select a string and use the Similar button in the editor to find heuristic matches. Results appear in the editor panel."
      />
    </div>
  );
}
