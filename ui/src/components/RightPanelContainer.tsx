import { Suspense, lazy } from "react";
import { useAppStore } from "../stores/appStore";
import { DockablePanel } from "./DockablePanel";
import { Loader } from "lucide-react";
import { useTranslation } from "react-i18next";

const BsaBrowser = lazy(() => import("./BsaBrowser").then(m => ({ default: m.BsaBrowser })));
const PexPanel = lazy(() => import("./PexPanel").then(m => ({ default: m.PexPanel })));
const FuzPanel = lazy(() => import("./FuzPanel").then(m => ({ default: m.FuzPanel })));
const EspComparePanel = lazy(() => import("./EspComparePanel").then(m => ({ default: m.EspComparePanel })));

const PANEL_TITLES: Record<string, string> = {
  bsa: "bsa.title",
  pex: "pex.title",
  fuz: "fuz.title",
  espCompare: "espCompare.title",
};

const PANEL_COMPONENTS: Record<string, React.ComponentType> = {
  bsa: BsaBrowser,
  pex: PexPanel,
  fuz: FuzPanel,
  espCompare: EspComparePanel,
};

export function RightPanelContainer() {
  const { t } = useTranslation();
  const activeRightPanel = useAppStore((s) => s.activeRightPanel);
  const setActiveRightPanel = useAppStore((s) => s.setActiveRightPanel);

  if (!activeRightPanel) return null;

  const PanelComponent = PANEL_COMPONENTS[activeRightPanel];
  if (!PanelComponent) return null;

  return (
    <DockablePanel
      title={t(PANEL_TITLES[activeRightPanel] || activeRightPanel)}
      onClose={() => setActiveRightPanel(null)}
    >
      <Suspense fallback={<div className="modal-loading"><Loader size={24} /></div>}>
        <PanelComponent />
      </Suspense>
    </DockablePanel>
  );
}
