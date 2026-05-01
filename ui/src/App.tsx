import { useEffect, useRef } from "react";
import { Toaster } from "react-hot-toast";
import toast from "react-hot-toast";
import { Loader } from "lucide-react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { setI18nLanguage } from "./i18n";
import { useAppStore } from "./stores/appStore";
import { MenuBar } from "./components/MenuBar";
import { SidePanel } from "./components/SidePanel";
import { BatchPanel } from "./components/BatchPanel";
import { BsaBrowser } from "./components/BsaBrowser";
import { PexPanel } from "./components/PexPanel";
import { FuzPanel } from "./components/FuzPanel";
import { DialogView } from "./components/DialogView";
import { McmPanel } from "./components/McmPanel";
import { EspComparePanel } from "./components/EspComparePanel";
import { FinalizePanel } from "./components/FinalizePanel";
import { DataConfigsPanel } from "./components/DataConfigsPanel";
import { StringTable } from "./components/StringTable";
import { EditorPanel } from "./components/EditorPanel";
import { BatchTranslateBar } from "./components/BatchTranslateBar";
import { RecoveryPromptModal } from "./components/RecoveryPromptModal";
import { autoBackupSst, loadConfig, setOpenAiApiKey, setDeeplApiKey, setTranslationProvider } from "./api/strings";
import "./App.css";

const AUTO_BACKUP_INTERVAL_MS = 5 * 60 * 1000;

type SidebarPanelId =
  | "mcm"
  | "espCompare"
  | "bsa"
  | "pex"
  | "fuz"
  | "dialog"
  | "batch"
  | "finalize"
  | "dataConfigs"
  | "overview";

type SidebarPanelFlags = {
  showMcmPanel: boolean;
  showEspCompare: boolean;
  showBsaBrowser: boolean;
  showPexPanel: boolean;
  showFuzPanel: boolean;
  showDialogView: boolean;
  showBatchPanel: boolean;
  showFinalizePanel: boolean;
  showDataConfigsPanel: boolean;
};

function getActiveSidebarPanel(flags: SidebarPanelFlags): SidebarPanelId {
  if (flags.showMcmPanel) return "mcm";
  if (flags.showEspCompare) return "espCompare";
  if (flags.showBsaBrowser) return "bsa";
  if (flags.showPexPanel) return "pex";
  if (flags.showFuzPanel) return "fuz";
  if (flags.showDialogView) return "dialog";
  if (flags.showBatchPanel) return "batch";
  if (flags.showFinalizePanel) return "finalize";
  if (flags.showDataConfigsPanel) return "dataConfigs";
  return "overview";
}

function renderSidebarPanel(panelId: SidebarPanelId) {
  switch (panelId) {
    case "mcm":
      return <McmPanel />;
    case "espCompare":
      return <EspComparePanel />;
    case "bsa":
      return <BsaBrowser />;
    case "pex":
      return <PexPanel />;
    case "fuz":
      return <FuzPanel />;
    case "dialog":
      return <DialogView />;
    case "batch":
      return <BatchPanel />;
    case "finalize":
      return <FinalizePanel />;
    case "dataConfigs":
      return <DataConfigsPanel />;
    case "overview":
      return <SidePanel />;
  }
}

function App() {
  const { t } = useTranslation();
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const isLoading = useAppStore((s) => s.isLoading);
  const isParsing = useAppStore((s) => s.isParsing);
  const loadProgress = useAppStore((s) => s.loadProgress);
  const theme = useAppStore((s) => s.theme);
  const showBatchPanel = useAppStore((s) => s.showBatchPanel);
  const showMcmPanel = useAppStore((s) => s.showMcmPanel);
  const showEspCompare = useAppStore((s) => s.showEspCompare);
  const showBsaBrowser = useAppStore((s) => s.showBsaBrowser);
  const showPexPanel = useAppStore((s) => s.showPexPanel);
  const showFuzPanel = useAppStore((s) => s.showFuzPanel);
  const showDialogView = useAppStore((s) => s.showDialogView);
  const showFinalizePanel = useAppStore((s) => s.showFinalizePanel);
  const showDataConfigsPanel = useAppStore((s) => s.showDataConfigsPanel);
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
    loadConfig().then((cfg) => {
      if (cfg.theme) useAppStore.getState().setTheme(cfg.theme as any);
      if (cfg.language) setI18nLanguage(cfg.language);
      if (cfg.openai_api_key) setOpenAiApiKey(cfg.openai_api_key);
      if (cfg.deepl_api_key) setDeeplApiKey(cfg.deepl_api_key);
      if (cfg.current_provider) setTranslationProvider(cfg.current_provider);
      if (cfg.esp_mode !== undefined) useAppStore.getState().setEspMode(cfg.esp_mode);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    reapplyTheme();
  }, [theme, reapplyTheme]);

  useEffect(() => {
    const unlisten = listen<{ str_id: number; translated: string; error: string | null; completed: number; total: number }>(
      "batch-string-progress",
      (event) => {
        const store = useAppStore.getState();
        store.updateItemTranslation(event.payload.str_id, event.payload.translated || "");
        useAppStore.setState({
          batchProgress: {
            completed: event.payload.completed,
            total: event.payload.total,
          },
        });
      }
    );

    const unlistenComplete = listen<{ total: number; succeeded: number; failed: number; errors: any[] }>(
      "batch-string-complete",
      (event) => {
        const { succeeded, failed } = event.payload;
        if (failed > 0) {
          toast(`Batch complete: ${succeeded} OK, ${failed} failed`);
        } else {
          toast.success(`Batch complete: ${succeeded} translated`);
        }
        useAppStore.setState({ batchState: "completed" });
      }
    );

    return () => {
      unlisten.then((u) => u());
      unlistenComplete.then((u) => u());
    };
  }, []);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      if (theme === "auto") reapplyTheme();
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme, reapplyTheme]);

  useEffect(() => {
    const interval = setInterval(async () => {
      if (!sstPath) return;
      if (isDirty) {
        try {
          const res = await autoBackupSst({ sst_path: sstPath, max_backups: 10 });
          if (res.backup_path) {
            backupIdRef.current = toast.success(
              `${t("toast.autoBackupSaved")} (${res.total_backups})`,
              { duration: 2000 }
            );
          }
        } catch {
          /* silent */
        }
      }
    }, AUTO_BACKUP_INTERVAL_MS);
    return () => {
      clearInterval(interval);
      if (backupIdRef.current !== null) toast.dismiss(backupIdRef.current);
    };
  }, [sstPath, isDirty, t]);

  const isLocked = isLoading || isParsing;
  const activeSidebarPanel = getActiveSidebarPanel({
    showMcmPanel,
    showEspCompare,
    showBsaBrowser,
    showPexPanel,
    showFuzPanel,
    showDialogView,
    showBatchPanel,
    showFinalizePanel,
    showDataConfigsPanel,
  });

  return (
    <div className="app">
      <Toaster position="top-right" />
      <RecoveryPromptModal />
      <MenuBar />
      <BatchTranslateBar />
      <div className="app-body">
        <aside className="app-sidebar" aria-label="Active side panel">
          {renderSidebarPanel(activeSidebarPanel)}
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
            {loadProgress?.message || (isParsing ? t("app.parsing") : t("app.processing"))}
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
