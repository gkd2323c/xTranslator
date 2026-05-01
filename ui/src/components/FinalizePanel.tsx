import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../stores/appStore";
import { finalize, finalizeEsp } from "../api/strings";
import type { FinalizeResponse, FinalizeEspResponse } from "../api/strings";
import toast from "react-hot-toast";
import { FileDown, Save, FolderOpen, HardDrive } from "lucide-react";
import { Button } from "./ui";

export function FinalizePanel() {
  const { t } = useTranslation();
  const espPath = useAppStore((s) => s.espPath);
  const targetLang = useAppStore((s) => s.targetLang);
  const allItems = useAppStore((s) => s.allItems);
  const isLoading = useAppStore((s) => s.isLoading);
  const setIsDirty = useAppStore((s) => s.setIsDirty);
  const espMode = useAppStore((s) => s.espMode);
  const language = useAppStore((s) => s.language);

  const [outputDir, setOutputDir] = useState<string>("");
  const [saveSst, setSaveSst] = useState(true);
  const [exportXml, setExportXml] = useState(true);
  const [sstPath, setSstPath] = useState<string>("");
  const [createBackup, setCreateBackup] = useState(true);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<FinalizeResponse | null>(null);
  const [espResult, setEspResult] = useState<FinalizeEspResponse | null>(null);

  // Compute stats from all items
  const stats = {
    total: allItems.length,
    translated: allItems.filter((s) => s.status === "translated").length,
    incomplete: allItems.filter((s) => s.status === "incomplete").length,
    locked: allItems.filter((s) => s.status === "locked").length,
  };

  const baseName = espPath
    ? espPath.replace(/\\/g, "/").split("/").pop()?.replace(/\.es[mp]$/i, "") || "Skyrim"
    : "Skyrim";

  const handleSelectOutputDir = async () => {
    const selected = await open({
      multiple: false,
      directory: true,
    });
    if (selected) {
      setOutputDir(Array.isArray(selected) ? selected[0] : selected);
    }
  };

  const handleSelectSstPath = async () => {
    const path = await save({
      filters: [{ name: "SST Dictionary", extensions: ["sst"] }],
      defaultPath: espPath
        ? espPath.replace(/\\/g, "/").replace(/\.es[mp]$/i, `_english_${targetLang}.sst`)
        : `translation_${targetLang}.sst`,
    });
    if (path) {
      setSstPath(path);
    }
  };

  const handleFinalize = async () => {
    if (!outputDir) {
      toast.error(t("finalize.needOutputDir"));
      return;
    }
    if (!espPath) {
      toast.error(t("finalize.needEsp"));
      return;
    }

    setRunning(true);
    try {
      const response = await finalize({
        strings_output_dir: outputDir,
        target_lang: targetLang,
        base_name: baseName,
        sst_path: saveSst ? sstPath : undefined,
        xml_path: exportXml
          ? `${outputDir.replace(/\\/g, "/").replace(/\/$/, "")}/${baseName}_${targetLang}.xml`
          : undefined,
      });
      setResult(response);
      setIsDirty(false);
      toast.success(
        t("finalize.success", {
          translated: response.translated_count,
          total: response.total_count,
        })
      );
    } catch (e: unknown) {
      toast.error(`${t("finalize.failed")}: ${e}`);
    } finally {
      setRunning(false);
    }
  };

  const handleFinalizeEsp = async () => {
    if (!espPath) {
      toast.error(t("finalize.needEsp"));
      return;
    }

    const stringsDir = espPath.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
    const baseName = espPath.replace(/\\/g, "/").split("/").pop()?.replace(/\.es[mp]$/i, "") || "Skyrim";

    setRunning(true);
    setEspResult(null);
    try {
      const response = await finalizeEsp({
        esp_path: espPath,
        strings_dir: stringsDir,
        base_name: baseName,
        language: language,
        create_backup: createBackup,
      });
      setEspResult(response);
      setIsDirty(false);
      toast.success(
        t("finalize.espSuccess", {
          defaultValue: "ESP finalized: {{count}} records modified",
          count: response.records_modified,
        })
      );
    } catch (e: unknown) {
      toast.error(`${t("finalize.failed")}: ${e}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="sidepanel">
      <div className="sidepanel-section">
        <h3>{t("finalize.title")}</h3>
        <p className="sidepanel-hint">
          {espMode
            ? t("finalize.espSubtitle", { defaultValue: "Write translations into ESP and export .STRINGS files" })
            : t("finalize.subtitle")}
        </p>
      </div>

      {/* Translation Summary */}
      <div className="sidepanel-section">
        <h4>{t("finalize.summary")}</h4>
        <div className="finalize-stats">
          <div className="finalize-stat">
            <span className="finalize-stat-value">{stats.total.toLocaleString()}</span>
            <span className="finalize-stat-label">{t("finalize.total")}</span>
          </div>
          <div className="finalize-stat finalize-stat-translated">
            <span className="finalize-stat-value">{stats.translated.toLocaleString()}</span>
            <span className="finalize-stat-label">{t("finalize.translated")}</span>
          </div>
          <div className="finalize-stat finalize-stat-incomplete">
            <span className="finalize-stat-value">{stats.incomplete.toLocaleString()}</span>
            <span className="finalize-stat-label">{t("finalize.incomplete")}</span>
          </div>
          <div className="finalize-stat finalize-stat-locked">
            <span className="finalize-stat-value">{stats.locked.toLocaleString()}</span>
            <span className="finalize-stat-label">{t("finalize.locked")}</span>
          </div>
        </div>
      </div>

      {/* Output Options — ESP mode vs Strings mode */}
      {espMode ? (
        <>
          <div className="sidepanel-section">
            <h4>{t("finalize.espOptions", { defaultValue: "ESP Write-back Options" })}</h4>
            <p className="sidepanel-hint">
              {t("finalize.espHint", { defaultValue: "Writes translations directly into the ESP file and exports .STRINGS files" })}
            </p>
            <div className="finalize-field">
              <label className="finalize-checkbox">
                <input
                  type="checkbox"
                  checked={createBackup}
                  onChange={(e) => setCreateBackup(e.target.checked)}
                />
                {t("finalize.createBackup", { defaultValue: "Create backup before writing" })}
              </label>
            </div>
          </div>

          <div className="sidepanel-section">
            <Button
              variant="primary"
              onClick={handleFinalizeEsp}
              disabled={running || isLoading || !espPath}
              icon={<HardDrive size={16} />}
              className="finalize-btn"
            >
              {running
                ? t("finalize.espExporting", { defaultValue: "Finalizing ESP..." })
                : t("finalize.espExportAll", { defaultValue: "Finalize ESP" })}
            </Button>
          </div>
        </>
      ) : (
        <>
          <div className="sidepanel-section">
            <h4>{t("finalize.outputOptions")}</h4>

            <div className="finalize-field">
              <label>{t("finalize.outputDirectory")}</label>
              <div className="finalize-field-row">
                <input
                  type="text"
                  value={outputDir}
                  placeholder={t("finalize.selectDirectory")}
                  className="input"
                  readOnly
                />
                <Button variant="default" size="sm" onClick={handleSelectOutputDir} icon={<FolderOpen size={14} />} />
              </div>
            </div>

            <div className="finalize-field">
              <label className="finalize-checkbox">
                <input
                  type="checkbox"
                  checked={saveSst}
                  onChange={(e) => setSaveSst(e.target.checked)}
                />
                {t("finalize.saveSst")}
              </label>
              {saveSst && (
                <div className="finalize-field-row" style={{ marginTop: 4 }}>
                  <input
                    type="text"
                    value={sstPath}
                    readOnly
                    placeholder={t("finalize.sstPathPlaceholder")}
                    className="input"
                  />
                  <Button variant="default" size="sm" onClick={handleSelectSstPath} icon={<Save size={14} />} />
                </div>
              )}
            </div>

            <div className="finalize-field">
              <label className="finalize-checkbox">
                <input
                  type="checkbox"
                  checked={exportXml}
                  onChange={(e) => setExportXml(e.target.checked)}
                />
                {t("finalize.exportXml")}
              </label>
            </div>
          </div>

          <div className="sidepanel-section">
            <Button
              variant="primary"
              onClick={handleFinalize}
              disabled={
                running ||
                isLoading ||
                !espPath ||
                !outputDir ||
                (saveSst && !sstPath)
              }
              icon={<FileDown size={16} />}
              className="finalize-btn"
            >
              {running ? t("finalize.exporting") : t("finalize.exportAll")}
            </Button>
          </div>
        </>
      )}

      {/* Result Output Files — Strings mode */}
      {!espMode && result && (
        <div className="sidepanel-section">
          <h4>{t("finalize.outputFiles")}</h4>
          <div className="finalize-files">
            {result.strings_path && (
              <div className="finalize-file">
                <span className="finalize-file-type">STRINGS</span>
                <span className="finalize-file-path" title={result.strings_path}>
                  {result.strings_path.split(/[/\\]/).pop()}
                </span>
              </div>
            )}
            {result.dlstrings_path && (
              <div className="finalize-file">
                <span className="finalize-file-type">DLSTRINGS</span>
                <span className="finalize-file-path" title={result.dlstrings_path}>
                  {result.dlstrings_path.split(/[/\\]/).pop()}
                </span>
              </div>
            )}
            {result.ilstrings_path && (
              <div className="finalize-file">
                <span className="finalize-file-type">ILSTRINGS</span>
                <span className="finalize-file-path" title={result.ilstrings_path}>
                  {result.ilstrings_path.split(/[/\\]/).pop()}
                </span>
              </div>
            )}
            {result.sst_path && (
              <div className="finalize-file">
                <span className="finalize-file-type">SST</span>
                <span className="finalize-file-path" title={result.sst_path}>
                  {result.sst_path.split(/[/\\]/).pop()}
                </span>
              </div>
            )}
            {result.xml_path && (
              <div className="finalize-file">
                <span className="finalize-file-type">XML</span>
                <span className="finalize-file-path" title={result.xml_path}>
                  {result.xml_path.split(/[/\\]/).pop()}
                </span>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Result — ESP mode */}
      {espMode && espResult && (
        <div className="sidepanel-section">
          <h4>{t("finalize.outputFiles")}</h4>
          <div className="finalize-files">
            <div className="finalize-file">
              <span className="finalize-file-type">ESP</span>
              <span className="finalize-file-path" title={espResult.esp_path}>
                {espResult.esp_path.split(/[/\\]/).pop()}
              </span>
            </div>
            {espResult.strings_files.map((f, i) => (
              <div className="finalize-file" key={i}>
                <span className="finalize-file-type">STRINGS</span>
                <span className="finalize-file-path" title={f}>
                  {f.split(/[/\\]/).pop()}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
