import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import toast from "react-hot-toast";
import { FileText, FolderOpen, Play, Save } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  readTextFile,
  runCommandProcessor,
  writeTextFile,
  type CommandProcessorErrorPolicy,
  type CommandProcessorProgress,
  type CommandProcessorRunResponse,
} from "../api/strings";
import { useAppStore } from "../stores/appStore";
import { Button, Input, Select, Textarea } from "./ui";
import "./CommandProcessorDialog.css";

const DRAFT_KEY = "xtranslator-command-processor-draft";

const DEFAULT_SCRIPT = `# Delphi-compatible xTranslator Command Processor
# Global_VocabFolder=C:\\Path\\To\\Vocab
# Global_ImportFolder=C:\\Path\\To\\Imports
# Global_ExportFolder=C:\\Path\\To\\Output

StartRule
LangSource=english
LangDest=chinese
UseDataDir=true
ExportSubFolder=Translated
Command=LoadFile:Example.esp
# Command=ApplySst:0:1:Example.esp
Command=Finalize
Command=CloseFile
EndRule
`;

export function CommandProcessorDialog() {
  const { t } = useTranslation();
  const currentGame = useAppStore((s) => s.currentGame);
  const [script, setScript] = useState(() => localStorage.getItem(DRAFT_KEY) ?? DEFAULT_SCRIPT);
  const [filePath, setFilePath] = useState("");
  const [dataDir, setDataDir] = useState("");
  const [errorPolicy, setErrorPolicy] = useState<CommandProcessorErrorPolicy>("stop");
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<CommandProcessorProgress[]>([]);
  const [result, setResult] = useState<CommandProcessorRunResponse | null>(null);

  useEffect(() => {
    localStorage.setItem(DRAFT_KEY, script);
  }, [script]);

  useEffect(() => {
    const unlisten = listen<CommandProcessorProgress>("command-processor-progress", (event) => {
      setProgress((current) => [...current.slice(-299), event.payload]);
    });
    return () => {
      unlisten.then((dispose) => dispose());
    };
  }, []);

  const openProcessor = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "xTranslator Processor", extensions: ["txt"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    try {
      setScript(await readTextFile(selected));
      setFilePath(selected);
    } catch (error) {
      toast.error(String(error));
    }
  };

  const saveProcessor = async () => {
    const target =
      filePath ||
      (await save({
        filters: [{ name: "xTranslator Processor", extensions: ["txt"] }],
        defaultPath: "processor.txt",
      }));
    if (!target) return;
    try {
      await writeTextFile(target, script);
      setFilePath(target);
      toast.success(t("commandProcessor.saved", { defaultValue: "Processor saved" }));
    } catch (error) {
      toast.error(String(error));
    }
  };

  const chooseDataDir = async () => {
    const selected = await open({ multiple: false, directory: true });
    if (selected && !Array.isArray(selected)) setDataDir(selected);
  };

  const runProcessor = async () => {
    if (!script.trim()) {
      toast.error(t("commandProcessor.empty", { defaultValue: "Processor script is empty" }));
      return;
    }

    setRunning(true);
    setProgress([]);
    setResult(null);
    try {
      const response = await runCommandProcessor({
        script,
        data_dir: dataDir || undefined,
        game: currentGame ?? undefined,
        error_policy: errorPolicy,
      });
      setResult(response);
      if (response.active_file) {
        useAppStore
          .getState()
          .setEspLoaded(response.active_file.esp_path, response.active_file.stats, response.active_file.strings_dir);
      } else if (response.file_context_changed) {
        useAppStore.getState().clearEspLoaded();
      }
      await useAppStore.getState().loadAllStrings();

      if (response.failures.length > 0) {
        toast.error(
          t("commandProcessor.completedWithErrors", {
            defaultValue: "Processor finished with {{count}} error(s)",
            count: response.failures.length,
          }),
        );
      } else {
        toast.success(
          t("commandProcessor.completed", {
            defaultValue: "Processor completed: {{count}} command(s)",
            count: response.commands_succeeded,
          }),
        );
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="command-processor">
      <div className="command-processor-toolbar">
        <div className="command-processor-actions">
          <Button size="sm" onClick={() => void openProcessor()} disabled={running}>
            <FileText size={14} />
            {t("commandProcessor.open", { defaultValue: "Open" })}
          </Button>
          <Button size="sm" onClick={() => void saveProcessor()} disabled={running}>
            <Save size={14} />
            {t("commandProcessor.save", { defaultValue: "Save" })}
          </Button>
          <span className="command-processor-file" title={filePath}>
            {filePath || t("commandProcessor.draft", { defaultValue: "Unsaved draft" })}
          </span>
        </div>
        <Button size="sm" onClick={() => void runProcessor()} disabled={running}>
          <Play size={14} />
          {running
            ? t("commandProcessor.running", { defaultValue: "Running…" })
            : t("commandProcessor.run", { defaultValue: "Run" })}
        </Button>
      </div>

      <div className="command-processor-options">
        <label className="command-processor-field command-processor-data-dir">
          <span>{t("commandProcessor.dataDir", { defaultValue: "Game Data directory" })}</span>
          <div className="command-processor-path-row">
            <Input
              value={dataDir}
              onChange={(event) => setDataDir(event.target.value)}
              placeholder={t("commandProcessor.dataDirHint", {
                defaultValue: "Required when UseDataDir=true",
              })}
              disabled={running}
            />
            <Button size="sm" onClick={() => void chooseDataDir()} disabled={running}>
              <FolderOpen size={14} />
            </Button>
          </div>
        </label>
        <label className="command-processor-field">
          <span>{t("commandProcessor.errorPolicy", { defaultValue: "On error" })}</span>
          <Select
            size="sm"
            value={errorPolicy}
            disabled={running}
            onChange={(event) => setErrorPolicy(event.target.value as CommandProcessorErrorPolicy)}
            options={[
              { value: "stop", label: t("commandProcessor.stop", { defaultValue: "Stop" }) },
              {
                value: "continue",
                label: t("commandProcessor.continue", { defaultValue: "Continue" }),
              },
            ]}
          />
        </label>
      </div>

      <div className="command-processor-main">
        <div className="command-processor-editor-wrap">
          <div className="command-processor-section-title">
            {t("commandProcessor.script", { defaultValue: "Processor script" })}
          </div>
          <Textarea
            className="command-processor-editor"
            value={script}
            onChange={(event) => setScript(event.target.value)}
            spellCheck={false}
            disabled={running}
          />
        </div>

        <div className="command-processor-log-wrap">
          <div className="command-processor-section-title">
            {t("commandProcessor.log", { defaultValue: "Execution log" })}
          </div>
          <div className="command-processor-log" role="log" aria-live="polite">
            {progress.length === 0 ? (
              <div className="command-processor-log-empty">
                {t("commandProcessor.logEmpty", { defaultValue: "No commands executed yet." })}
              </div>
            ) : (
              progress.map((entry, index) => (
                <div key={`${entry.rule_number}-${entry.command_number ?? 0}-${entry.line}-${index}`}>
                  <span className="command-processor-log-pos">
                    R{entry.rule_number}
                    {entry.command_number ? ` C${entry.command_number}` : ""} · L{entry.line}
                  </span>{" "}
                  {entry.message}
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      {result && (
        <div className="command-processor-result">
          <strong>
            {t("commandProcessor.result", { defaultValue: "Result" })}: {result.commands_succeeded}{" "}
            {t("commandProcessor.commandsSucceeded", { defaultValue: "commands succeeded" })}
          </strong>
          <span>
            {result.rules_completed}/{result.rules_started}{" "}
            {t("commandProcessor.rulesCompleted", { defaultValue: "rules completed" })}
          </span>
          {result.failures.map((failure, index) => (
            <div className="command-processor-failure" key={`failure-${index}`}>
              R{failure.rule_number}
              {failure.command_number ? ` C${failure.command_number}` : ""} · L{failure.line}
              {failure.command ? ` · ${failure.command}` : ""}: {failure.message}
            </div>
          ))}
          {result.warnings.map((warning, index) => (
            <div className="command-processor-warning" key={`warning-${index}`}>
              {warning}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
