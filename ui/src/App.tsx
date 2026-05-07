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
import { VocabularyPanel } from "./components/bottom/VocabularyPanel";
import { HeuristicPanel } from "./components/bottom/HeuristicPanel";
import { EspTreePanel } from "./components/bottom/EspTreePanel";
import { QuestsPanel } from "./components/bottom/QuestsPanel";
import { LogPanel } from "./components/bottom/LogPanel";
import { HeaderProcessorPanel } from "./components/bottom/HeaderProcessorPanel";
import { HeaderWizardPanel } from "./components/bottom/HeaderWizardPanel";
import { StringTable } from "./components/StringTable";
import { EditorDialog } from "./components/EditorPanel";
import { BatchTranslateBar } from "./components/BatchTranslateBar";
import { RecoveryPromptModal } from "./components/RecoveryPromptModal";
import { StatusBar } from "./components/StatusBar";
import { Modal } from "./components/ui";
import { autoBackupSst, loadConfig, setOpenAiApiKey, setDeeplApiKey, setBaiduApiKey, setYoudaoApiKey, setAzureApiKey, setTranslationProvider } from "./api/strings";
import "./App.css";
import "./components/ui/ui.css";

const AUTO_BACKUP_INTERVAL_MS = 5 * 60 * 1000;

function App() {
  const { t } = useTranslation();
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const isLoading = useAppStore((s) => s.isLoading);
  const isParsing = useAppStore((s) => s.isParsing);
  const loadProgress = useAppStore((s) => s.loadProgress);
  const theme = useAppStore((s) => s.theme);
  const activePanel = useAppStore((s) => s.activePanel);
  const setActivePanel = useAppStore((s) => s.setActivePanel);
  const activeBottomTab = useAppStore((s) => s.activeBottomTab);
  const showBottomPanel = useAppStore((s) => s.showBottomPanel);
  const editorOpen = useAppStore((s) => s.editorOpen);
  const setEditorOpen = useAppStore((s) => s.setEditorOpen);
  const isDirty = useAppStore((s) => s.isDirty);
  const sstPath = useAppStore((s) => s.sstPath);
  const undo = useAppStore((s) => s.undo);
  const redo = useAppStore((s) => s.redo);
  const reapplyTheme = useAppStore((s) => s.reapplyTheme);
  const backupIdRef = useRef<string | null>(null);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (editorOpen) {
          setEditorOpen(false);
        } else if (activePanel) {
          setActivePanel(null);
        } else {
          setSelectedById(null);
        }
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
  }, [setSelectedById, undo, redo, editorOpen, setEditorOpen, activePanel, setActivePanel]);

  useEffect(() => {
    loadConfig().then((cfg) => {
      if (cfg.theme) useAppStore.getState().setTheme(cfg.theme as any);
      if (cfg.language) setI18nLanguage(cfg.language);
      if (cfg.openai_api_key) setOpenAiApiKey(cfg.openai_api_key);
      if (cfg.deepl_api_key) setDeeplApiKey(cfg.deepl_api_key);
      if (cfg.baidu_app_id && cfg.baidu_key) setBaiduApiKey(cfg.baidu_app_id, cfg.baidu_key);
      if (cfg.youdao_app_key && cfg.youdao_secret_key) setYoudaoApiKey(cfg.youdao_app_key, cfg.youdao_secret_key);
      if (cfg.azure_key) setAzureApiKey(cfg.azure_key);
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
          toast(t("batch.completeWithFailures", { succeeded, failed }));
        } else {
          toast.success(t("batch.completeSuccess", { succeeded }));
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

  return (
    <div className="app">
      <Toaster position="top-right" />
      <RecoveryPromptModal />
      <EditorDialog open={editorOpen} onClose={() => setEditorOpen(false)} />
      <Modal open={activePanel === "batch"} onClose={() => setActivePanel(null)} title={t("batch.title")} size="lg"><BatchPanel /></Modal>
      <Modal open={activePanel === "bsa"} onClose={() => setActivePanel(null)} title={t("bsa.title")} size="lg"><BsaBrowser /></Modal>
      <Modal open={activePanel === "pex"} onClose={() => setActivePanel(null)} title={t("pex.title")} size="lg"><PexPanel /></Modal>
      <Modal open={activePanel === "fuz"} onClose={() => setActivePanel(null)} title={t("fuz.title")} size="lg"><FuzPanel /></Modal>
      <Modal open={activePanel === "dialog"} onClose={() => setActivePanel(null)} title={t("dialog.title")} size="lg"><DialogView /></Modal>
      <Modal open={activePanel === "mcm"} onClose={() => setActivePanel(null)} title={t("mcm.title")} size="lg"><McmPanel /></Modal>
      <Modal open={activePanel === "espCompare"} onClose={() => setActivePanel(null)} title={t("espCompare.title")} size="lg"><EspComparePanel /></Modal>
      <Modal open={activePanel === "finalize"} onClose={() => setActivePanel(null)} title={t("finalize.title")} size="lg"><FinalizePanel /></Modal>
      <Modal open={activePanel === "dataConfigs"} onClose={() => setActivePanel(null)} title={t("dataConfigs.title")} size="lg"><DataConfigsPanel /></Modal>
      <MenuBar />
      <BatchTranslateBar />
      <div className="app-body">
        <main className="app-main">
          {/* String table area */}
          <div className="app-table-area">
            <StringTable />
          </div>
          {/* Bottom panel (tabbed auxiliary views) */}
          {showBottomPanel && (
            <>
              <div className="app-bottom-splitter" />
              <div className="app-bottom-panel">
                <div className="bottom-panel-tabs">
                  {(["home", "vocabulary", "heuristic", "espTree", "pex", "quests", "dialogs", "log", "headerProc", "headerWizard"] as const).map((tab) => (
                    <button
                      key={tab}
                      className={`bottom-tab ${activeBottomTab === tab ? "bottom-tab-active" : ""}`}
                      onClick={() => useAppStore.getState().setActiveBottomTab(tab)}
                    >
                      {t(`bottomTabs.${tab}`)}
                    </button>
                  ))}

                </div>
                <div className="bottom-panel-content">
                  {activeBottomTab === "home" && <SidePanel />}
                  {activeBottomTab === "vocabulary" && <VocabularyPanel />}
                  {activeBottomTab === "heuristic" && <HeuristicPanel />}
                  {activeBottomTab === "espTree" && <EspTreePanel />}
                  {activeBottomTab === "pex" && <PexPanel />}
                  {activeBottomTab === "quests" && <QuestsPanel />}
                  {activeBottomTab === "dialogs" && <DialogView />}
                  {activeBottomTab === "log" && <LogPanel />}
                  {activeBottomTab === "headerProc" && <HeaderProcessorPanel />}
                  {activeBottomTab === "headerWizard" && <HeaderWizardPanel />}
                </div>
              </div>
            </>
          )}
        </main>
      </div>
      <StatusBar />
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
