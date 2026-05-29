import { useState } from "react";
import { HeaderProcessorPanel } from "./HeaderProcessorPanel";
import { HeaderWizardPanel } from "./HeaderWizardPanel";
import { useTranslation } from "react-i18next";

type SubTab = "processor" | "wizard";

export function HeaderTab() {
  const { t } = useTranslation();
  const [subTab, setSubTab] = useState<SubTab>("processor");

  return (
    <div className="header-tab">
      <div className="sub-tab-bar">
        <button
          className={`sub-tab ${subTab === "processor" ? "sub-tab-active" : ""}`}
          onClick={() => setSubTab("processor")}
        >
          {t("bottomTabs.headerProc")}
        </button>
        <button
          className={`sub-tab ${subTab === "wizard" ? "sub-tab-active" : ""}`}
          onClick={() => setSubTab("wizard")}
        >
          {t("bottomTabs.headerWizard")}
        </button>
      </div>
      <div className="sub-tab-content">
        {subTab === "processor" && <HeaderProcessorPanel />}
        {subTab === "wizard" && <HeaderWizardPanel />}
      </div>
    </div>
  );
}
