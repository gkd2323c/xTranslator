import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../stores/appStore";
import { finalize } from "../api/strings";
import type { FinalizeResponse } from "../api/strings";
import toast from "react-hot-toast";
import { FileDown, Save, FolderOpen } from "lucide-react";

export function FinalizePanel() {
  const { t } = useTranslation();
  const espPath = useAppStore((s) => s.espPath);
  const targetLang = useAppStore((s) => s.targetLang);
  const allItems = useAppStore((s) => s.allItems);
  const isLoading = useAppStore((s) => s.isLoading);
  const setIsDirty = useAppStore((s) => s.setIsDirty);

  const [outputDir, setOutputDir] = useState<string>("");
  const [saveSst, setSaveSst] = useState(true);
  const [exportXml, setExportXml] = useState(true);
  const [sstPath, setSstPath] = useState<string>("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<FinalizeResponse | null>(null);

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

  return (
    <div className="sidepanel">
      <div className="sidepanel-section">
        <h3>{t("finalize.title")}</h3>
        <p className="sidepanel-hint">{t("finalize.subtitle")}</p>
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

      {/* Output Options */}
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
            <button onClick={handleSelectOutputDir} className="btn btn-sm">
              <FolderOpen size={14} />
            </button>
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
              <button onClick={handleSelectSstPath} className="btn btn-sm">
                <Save size={14} />
              </button>
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

      {/* Action */}
      <div className="sidepanel-section">
        <button
          onClick={handleFinalize}
          disabled={
            running ||
            isLoading ||
            !espPath ||
            !outputDir ||
            (saveSst && !sstPath)
          }
          className="btn btn-primary"
          style={{ width: "100%" }}
        >
          <FileDown size={16} />
          <span>{running ? t("finalize.exporting") : t("finalize.exportAll")}</span>
        </button>
      </div>

      {/* Result Output Files */}
      {result && (
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
    </div>
  );
}
