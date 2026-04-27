import { useEffect, useRef } from "react";
import { Toaster } from "react-hot-toast";
import toast from "react-hot-toast";
import { Loader } from "lucide-react";
import { useAppStore } from "./stores/appStore";
import { MenuBar } from "./components/MenuBar";
import { SidePanel } from "./components/SidePanel";
import { BatchPanel } from "./components/BatchPanel";
import { StringTable } from "./components/StringTable";
import { EditorPanel } from "./components/EditorPanel";
import { autoBackupSst } from "./api/strings";
import "./App.css";

const AUTO_BACKUP_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

function App() {
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const isLoading = useAppStore((s) => s.isLoading);
  const isParsing = useAppStore((s) => s.isParsing);
  const loadProgress = useAppStore((s) => s.loadProgress);
  const theme = useAppStore((s) => s.theme);
  const showBatchPanel = useAppStore((s) => s.showBatchPanel);
  const isDirty = useAppStore((s) => s.isDirty);
  const sstPath = useAppStore((s) => s.sstPath);
  const undo = useAppStore((s) => s.undo);
  const redo = useAppStore((s) => s.redo);
  const reapplyTheme = useAppStore((s) => s.reapplyTheme);
  const backupIdRef = useRef<string | null>(null);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setSelectedById(null);
      } else if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
      } else if ((e.ctrlKey || e.metaKey) && (e.key === "y" || (e.key === "z" && e.shiftKey))) {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setSelectedById, undo, redo]);

  useEffect(() => {
    reapplyTheme();
  }, [theme, reapplyTheme]);

  // Listen for system color scheme changes when in "auto" mode
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      if (theme === "auto") reapplyTheme();
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme, reapplyTheme]);

  // Auto-backup timer: saves SST snapshot every 5 minutes when dirty
  useEffect(() => {
    const interval = setInterval(async () => {
      if (!sstPath) return;
      if (isDirty) {
        try {
          const res = await autoBackupSst({ sst_path: sstPath, max_backups: 10 });
          if (res.backup_path) {
            backupIdRef.current = toast.success(
              `Auto-backup saved (${res.total_backups} total)`,
              { duration: 2000 }
            );
          }
        } catch {
          // Silent fail — don't interrupt user for backup errors
        }
      }
    }, AUTO_BACKUP_INTERVAL_MS);

    return () => {
      clearInterval(interval);
      if (backupIdRef.current !== null) toast.dismiss(backupIdRef.current);
    };
  }, [sstPath, isDirty]);

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
