import { useState, useEffect } from "react";
import { loadConfig, saveConfig, setOpenAiApiKey, setDeeplApiKey, setTranslationProvider, getTranslationProviders, type AppConfigDto, type TranslationProvidersResponse } from "../api/strings";
import { Settings, X } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { t } = useTranslation();
  const [config, setConfig] = useState<AppConfigDto>({});
  const [providers, setProviders] = useState<TranslationProvidersResponse | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      loadConfig().then(setConfig).catch(() => {});
      getTranslationProviders().then(setProviders).catch(() => {});
    }
  }, [open]);

  if (!open) return null;

  const handleChange = (field: keyof AppConfigDto, value: string | number | undefined) => {
    setConfig((prev) => ({ ...prev, [field]: value || undefined }));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      // Save proxy settings to config
      await saveConfig(config);

      // Save API keys via IPC (in-memory)
      if (config.openai_api_key !== undefined) {
        await setOpenAiApiKey(config.openai_api_key);
      }
      if (config.deepl_api_key !== undefined) {
        await setDeeplApiKey(config.deepl_api_key);
      }
      if (config.current_provider) {
        await setTranslationProvider(config.current_provider);
      }

      toast.success(t("settings.saved", { defaultValue: "Settings saved" }));
      onClose();
    } catch (e: any) {
      toast.error(`${t("settings.saveFailed", { defaultValue: "Failed to save settings" })}: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog-content dialog-wide"
        onClick={(e) => e.stopPropagation()}
        style={{ minWidth: 480, maxWidth: 560 }}
      >
        <div className="dialog-header">
          <h3><Settings size={18} /> {t("settings.title", { defaultValue: "Settings" })}</h3>
          <button onClick={onClose} className="btn btn-ghost btn-sm"><X size={16} /></button>
        </div>

        {/* API Keys */}
        <div className="dialog-section">
          <h4>{t("settings.apiKeys", { defaultValue: "Translation API" })}</h4>

          <label className="dialog-label">
            {t("settings.provider", { defaultValue: "Provider" })}
          </label>
          <select
            value={config.current_provider || providers?.current || "openai"}
            onChange={(e) => handleChange("current_provider", e.target.value)}
            className="dialog-input"
          >
            <option value="openai">OpenAI</option>
            <option value="deepl">DeepL</option>
          </select>

          <label className="dialog-label">
            OpenAI API Key
          </label>
          <input
            type="password"
            value={config.openai_api_key || ""}
            onChange={(e) => handleChange("openai_api_key", e.target.value)}
            placeholder="sk-..."
            className="dialog-input"
          />
          <p className="dialog-hint">
            {t("settings.apiKeyHint", { defaultValue: "Supports OpenAI-compatible APIs (OpenAI, DeepSeek, etc.). Also settable via XT_TRANSLATE_API_KEY env var." })}
          </p>

          <label className="dialog-label">
            DeepL API Key
          </label>
          <input
            type="password"
            value={config.deepl_api_key || ""}
            onChange={(e) => handleChange("deepl_api_key", e.target.value)}
            placeholder={t("settings.deeplPlaceholder", { defaultValue: "DeepL API key (free: xxx:fx, pro: xxx)" })}
            className="dialog-input"
          />
        </div>

        {/* Proxy */}
        <div className="dialog-section">
          <h4>{t("settings.proxy", { defaultValue: "HTTP Proxy" })}</h4>

          <label className="dialog-label">
            {t("settings.proxyServer", { defaultValue: "Proxy Server" })}
          </label>
          <input
            type="text"
            value={config.proxy_server || ""}
            onChange={(e) => handleChange("proxy_server", e.target.value)}
            placeholder={t("settings.proxyServerPlaceholder", { defaultValue: "e.g. proxy.example.com (leave empty to disable)" })}
            className="dialog-input"
          />

          <label className="dialog-label">
            {t("settings.proxyPort", { defaultValue: "Port" })}
          </label>
          <input
            type="number"
            value={config.proxy_port ?? 8080}
            onChange={(e) => handleChange("proxy_port", parseInt(e.target.value, 10) || undefined)}
            placeholder="8080"
            className="dialog-input"
            min={1}
            max={65535}
          />

          <label className="dialog-label">
            {t("settings.proxyUsername", { defaultValue: "Username (optional)" })}
          </label>
          <input
            type="text"
            value={config.proxy_username || ""}
            onChange={(e) => handleChange("proxy_username", e.target.value)}
            placeholder={t("settings.proxyAuthPlaceholder", { defaultValue: "Leave empty if no auth" })}
            className="dialog-input"
          />

          <label className="dialog-label">
            {t("settings.proxyPassword", { defaultValue: "Password (optional)" })}
          </label>
          <input
            type="password"
            value={config.proxy_password || ""}
            onChange={(e) => handleChange("proxy_password", e.target.value)}
            placeholder={t("settings.proxyAuthPlaceholder", { defaultValue: "Leave empty if no auth" })}
            className="dialog-input"
          />
          <p className="dialog-hint">
            {t("settings.proxyHint", { defaultValue: "Proxy settings apply to OpenAI and DeepL translation requests. Saved to config file." })}
          </p>
        </div>

        <div className="dialog-actions">
          <button onClick={onClose} className="btn btn-ghost">
            {t("common.cancel")}
          </button>
          <button onClick={handleSave} disabled={saving} className="btn btn-primary">
            {saving ? t("settings.saving", { defaultValue: "Saving..." }) : t("common.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}
