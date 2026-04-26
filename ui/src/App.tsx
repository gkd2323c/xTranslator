import { useEffect } from "react";
import { Toaster } from "react-hot-toast";
import { Loader } from "lucide-react";
import { useAppStore } from "./stores/appStore";
import { MenuBar } from "./components/MenuBar";
import { SidePanel } from "./components/SidePanel";
import { BatchPanel } from "./components/BatchPanel";
import { StringTable } from "./components/StringTable";
import { EditorPanel } from "./components/EditorPanel";
import "./App.css";

function App() {
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const isLoading = useAppStore((s) => s.isLoading);
  const isParsing = useAppStore((s) => s.isParsing);
  const loadProgress = useAppStore((s) => s.loadProgress);
  const theme = useAppStore((s) => s.theme);
  const showBatchPanel = useAppStore((s) => s.showBatchPanel);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setSelectedById(null);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setSelectedById]);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  const isLocked = isLoading || isParsing;

  return (
    <div className="app">
      <Toaster position="top-right" />
      <MenuBar />
      <div className="app-body">
        <aside className="app-sidebar">
          {showBatchPanel ? <BatchPanel /> : <SidePanel />}
        </aside>
        <main className="app-main">
          <div className="app-table-area">
            <StringTable />
          </div>
          <div className="app-editor-area">
            <EditorPanel />
          </div>
        </main>
      </div>
      {isLocked && (
        <div className="app-overlay">
          <Loader size={40} className="app-overlay-spinner" />
          <p className="app-overlay-message">
            {loadProgress?.message || (isParsing ? "Parsing ESP..." : "Processing...")}
          </p>
          {loadProgress && loadProgress.total > 0 && (
            <div className="app-overlay-progress">
              <div
                className="app-overlay-progress-bar"
                style={{ width: `${loadProgress.percentage}%` }}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default App;
