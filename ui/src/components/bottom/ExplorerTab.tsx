import { useState } from "react";
import { HeuristicPanel } from "./HeuristicPanel";
import { EspTreePanel } from "./EspTreePanel";
import { QuestsPanel } from "./QuestsPanel";
import { useTranslation } from "react-i18next";

type SubTab = "heuristic" | "espTree" | "quests";

export function ExplorerTab() {
  const { t } = useTranslation();
  const [subTab, setSubTab] = useState<SubTab>("heuristic");

  return (
    <div className="explorer-tab">
      <div className="sub-tab-bar">
        {(["heuristic", "espTree", "quests"] as const).map((tab) => (
          <button
            key={tab}
            className={`sub-tab ${subTab === tab ? "sub-tab-active" : ""}`}
            onClick={() => setSubTab(tab)}
          >
            {t(`bottomTabs.${tab}`)}
          </button>
        ))}
      </div>
      <div className="sub-tab-content">
        {subTab === "heuristic" && <HeuristicPanel />}
        {subTab === "espTree" && <EspTreePanel />}
        {subTab === "quests" && <QuestsPanel />}
      </div>
    </div>
  );
}
