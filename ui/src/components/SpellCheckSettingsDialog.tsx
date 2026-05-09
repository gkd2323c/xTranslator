import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { spellCheckLoad, spellCheckUnload, spellCheckToggle, spellCheckConfig, type SpellCheckConfigDto } from "../api/strings";
import toast from "react-hot-toast";
import { Button, Modal, Select } from "./ui";
import { BookOpen, BookX, RefreshCw, ToggleLeft, ToggleRight } from "lucide-react";

interface SpellCheckSettingsDialogProps {
  open: boolean;
  onClose: () => void;
  dllPath: string;
  dictDir: string;
  onConfigChanged: (config: SpellCheckConfigDto | null) => void;
}

export function SpellCheckSettingsDialog({ open, onClose, dllPath, dictDir, onConfigChanged }: SpellCheckSettingsDialogProps) {
  const { t } = useTranslation();
  const [config, setConfig] = useState<SpellCheckConfigDto | null>(null);
  const [selectedDict, setSelectedDict] = useState("");
  const [loading, setLoading] = useState(false);
  const [scanning, setScanning] = useState(false);

  const fetchConfig = useCallback(async () => {
    try {
      const cfg = await spellCheckConfig(dictDir);
      setConfig(cfg);
      if (cfg.current_dictionary) {
        setSelectedDict(cfg.current_dictionary);
      }
      onConfigChanged(cfg);
    } catch (e: any) {
      // silently fail - dictionaries dir might not exist yet
      setConfig(null);
    }
  }, [dictDir, onConfigChanged]);

  useEffect(() => {
    if (open) {
      fetchConfig();
    }
  }, [open, fetchConfig]);

  const handleLoad = async () => {
    if (!selectedDict) {
      toast.error(t("spellcheck.noDictSelected", { defaultValue: "No dictionary selected" }));
      return;
    }
    setLoading(true);
    try {
      const cfg = await spellCheckLoad(dllPath, dictDir, selectedDict);
      setConfig(cfg);
      onConfigChanged(cfg);
      toast.success(t("spellcheck.loaded", { defaultValue: "Spell checker loaded" }));
    } catch (e: any) {
      toast.error(`${t("spellcheck.loadFailed", { defaultValue: "Failed to load spell checker" })}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleUnload = async () => {
    try {
      await spellCheckUnload();
      const cfg = await spellCheckConfig(dictDir);
      setConfig(cfg);
      onConfigChanged(cfg);
      toast.success(t("spellcheck.unloaded", { defaultValue: "Spell checker unloaded" }));
    } catch (e: any) {
      toast.error(`${t("spellcheck.unloadFailed", { defaultValue: "Failed to unload" })}: ${e}`);
    }
  };

  const handleToggle = async () => {
    try {
      const active = await spellCheckToggle();
      setConfig((prev) => prev ? { ...prev, active } : prev);
      if (config) {
        onConfigChanged({ ...config, active });
      }
    } catch (e: any) {
      toast.error(`${t("spellcheck.toggleFailed", { defaultValue: "Toggle failed" })}: ${e}`);
    }
  };

  const handleScan = async () => {
    setScanning(true);
    await fetchConfig();
    setScanning(false);
  };

  const dictOptions = (config?.available_dictionaries || []).map((d) => ({
    value: d,
    label: d,
  }));

  const isLoaded = config?.loaded ?? false;
  const isActive = config?.active ?? false;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("spellcheck.title", { defaultValue: "Spell Check Settings" })}
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.close")}
          </Button>
        </>
      }
    >
      <div className="spellcheck-settings">
        {/* Status */}
        <div className="spellcheck-status-row">
          <span className="spellcheck-label">{t("spellcheck.status", { defaultValue: "Status" })}:</span>
          <span className={`spellcheck-status-value ${isLoaded ? (isActive ? "spellcheck-active" : "spellcheck-loaded") : "spellcheck-inactive"}`}>
            {!isLoaded
              ? t("spellcheck.notLoaded", { defaultValue: "Not loaded" })
              : isActive
                ? t("spellcheck.running", { defaultValue: "Active" })
                : t("spellcheck.idle", { defaultValue: "Loaded (idle)" })}
          </span>
        </div>

        {/* Dictionary Selection */}
        <div className="spellcheck-field">
          <label className="spellcheck-label">{t("spellcheck.dictionary", { defaultValue: "Dictionary" })}:</label>
          <div className="spellcheck-field-row">
            <Select
              options={dictOptions.length > 0 ? dictOptions : [{ value: "", label: t("spellcheck.noDictsFound", { defaultValue: "No dictionaries found" }) }]}
              value={selectedDict}
              onChange={(e) => setSelectedDict(e.target.value)}
              className="spellcheck-select"
            />
            <Button variant="ghost" size="sm" onClick={handleScan} loading={scanning} icon={<RefreshCw size={14} />}>
              {t("spellcheck.scan", { defaultValue: "Scan" })}
            </Button>
          </div>
        </div>

        {/* Paths (read-only info) */}
        <div className="spellcheck-field">
          <label className="spellcheck-label">{t("spellcheck.dllPath", { defaultValue: "Hunspell DLL" })}:</label>
          <code className="spellcheck-path">{dllPath}</code>
        </div>
        <div className="spellcheck-field">
          <label className="spellcheck-label">{t("spellcheck.dictDir", { defaultValue: "Dictionaries folder" })}:</label>
          <code className="spellcheck-path">{dictDir}</code>
        </div>

        {/* Actions */}
        <div className="spellcheck-actions">
          {!isLoaded ? (
            <Button variant="primary" size="sm" onClick={handleLoad} loading={loading} icon={<BookOpen size={14} />}>
              {t("spellcheck.load", { defaultValue: "Load" })}
            </Button>
          ) : (
            <Button variant="ghost" size="sm" onClick={handleUnload} icon={<BookX size={14} />}>
              {t("spellcheck.unload", { defaultValue: "Unload" })}
            </Button>
          )}
          {isLoaded && (
            <Button
              variant={isActive ? "primary" : "ghost"}
              size="sm"
              onClick={handleToggle}
              icon={isActive ? <ToggleRight size={14} /> : <ToggleLeft size={14} />}
            >
              {isActive
                ? t("spellcheck.deactivate", { defaultValue: "Deactivate" })
                : t("spellcheck.activate", { defaultValue: "Activate" })}
            </Button>
          )}
        </div>

        {/* Help text */}
        <p className="spellcheck-help">
          {t("spellcheck.helpText", {
            defaultValue:
              "Place .dic and .aff dictionary files from OpenOffice/Mozilla in the dictionaries folder, then scan to detect them.",
          })}
        </p>
      </div>
    </Modal>
  );
}
