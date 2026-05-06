import { useState, useEffect } from "react";
import { loadConfig, saveConfig, setOpenAiApiKey, setDeeplApiKey, setBaiduApiKey, setYoudaoApiKey, setAzureApiKey, setTranslationProvider, getTranslationProviders, type AppConfigDto, type TranslationProvidersResponse } from "../api/strings";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { Button, Modal, Input, Select } from "./ui";

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
      if (config.baidu_app_id && config.baidu_key) {
        await setBaiduApiKey(config.baidu_app_id, config.baidu_key);
      }
      if (config.youdao_app_key && config.youdao_secret_key) {
        await setYoudaoApiKey(config.youdao_app_key, config.youdao_secret_key);
      }
      if (config.azure_key) {
        await setAzureApiKey(config.azure_key);
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
    <Modal
      open={open}
      onClose={onClose}
      title={t("settings.title", { defaultValue: "Settings" })}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={handleSave} loading={saving}>
            {saving ? t("settings.saving", { defaultValue: "Saving..." }) : t("common.confirm")}
          </Button>
        </>
      }
    >
      {/* API Keys */}
      <div className="dialog-section">
        <h4>{t("settings.apiKeys", { defaultValue: "Translation API" })}</h4>

        <label className="dialog-label">
          {t("settings.provider", { defaultValue: "Provider" })}
        </label>
        <Select
          value={config.current_provider || providers?.current || "openai"}
          onChange={(e) => handleChange("current_provider", e.target.value)}
          options={[
            { value: "openai", label: "OpenAI" },
            { value: "deepl", label: "DeepL" },
            { value: "baidu", label: "Baidu" },
            { value: "youdao", label: "Youdao" },
            { value: "azure", label: "Azure" },
            { value: "google", label: "Google" },
          ]}
        />

        <label className="dialog-label">
          OpenAI API Key
        </label>
        <Input
          type="password"
          value={config.openai_api_key || ""}
          onChange={(e) => handleChange("openai_api_key", e.target.value)}
          placeholder="sk-..."
        />
        <p className="ui-modal-hint">
          {t("settings.apiKeyHint", { defaultValue: "Supports OpenAI-compatible APIs (OpenAI, DeepSeek, etc.). Also settable via XT_TRANSLATE_API_KEY env var." })}
        </p>

        <label className="dialog-label">
          DeepL API Key
        </label>
        <Input
          type="password"
          value={config.deepl_api_key || ""}
          onChange={(e) => handleChange("deepl_api_key", e.target.value)}
          placeholder={t("settings.deeplPlaceholder", { defaultValue: "DeepL API key (free: xxx:fx, pro: xxx)" })}
        />

        <label className="dialog-label">
          Baidu App ID
        </label>
        <Input
          type="text"
          value={config.baidu_app_id || ""}
          onChange={(e) => handleChange("baidu_app_id", e.target.value)}
          placeholder="Baidu AppId"
        />

        <label className="dialog-label">
          Baidu Key
        </label>
        <Input
          type="password"
          value={config.baidu_key || ""}
          onChange={(e) => handleChange("baidu_key", e.target.value)}
          placeholder="Baidu secret key"
        />

        <label className="dialog-label">
          Youdao App Key
        </label>
        <Input
          type="text"
          value={config.youdao_app_key || ""}
          onChange={(e) => handleChange("youdao_app_key", e.target.value)}
          placeholder="Youdao AppKey"
        />

        <label className="dialog-label">
          Youdao Secret Key
        </label>
        <Input
          type="password"
          value={config.youdao_secret_key || ""}
          onChange={(e) => handleChange("youdao_secret_key", e.target.value)}
          placeholder="Youdao SecretKey"
        />

        <label className="dialog-label">
          Azure Key
        </label>
        <Input
          type="password"
          value={config.azure_key || ""}
          onChange={(e) => handleChange("azure_key", e.target.value)}
          placeholder="Azure subscription key"
        />
      </div>

      {/* Proxy */}
      <div className="dialog-section">
        <h4>{t("settings.proxy", { defaultValue: "HTTP Proxy" })}</h4>

        <label className="dialog-label">
          {t("settings.proxyServer", { defaultValue: "Proxy Server" })}
        </label>
        <Input
          type="text"
          value={config.proxy_server || ""}
          onChange={(e) => handleChange("proxy_server", e.target.value)}
          placeholder={t("settings.proxyServerPlaceholder", { defaultValue: "e.g. proxy.example.com (leave empty to disable)" })}
        />

        <label className="dialog-label">
          {t("settings.proxyPort", { defaultValue: "Port" })}
        </label>
        <Input
          type="number"
          value={config.proxy_port ?? 8080}
          onChange={(e) => handleChange("proxy_port", parseInt(e.target.value, 10) || undefined)}
          placeholder="8080"
        />

        <label className="dialog-label">
          {t("settings.proxyUsername", { defaultValue: "Username (optional)" })}
        </label>
        <Input
          type="text"
          value={config.proxy_username || ""}
          onChange={(e) => handleChange("proxy_username", e.target.value)}
          placeholder={t("settings.proxyAuthPlaceholder", { defaultValue: "Leave empty if no auth" })}
        />

        <label className="dialog-label">
          {t("settings.proxyPassword", { defaultValue: "Password (optional)" })}
        </label>
        <Input
          type="password"
          value={config.proxy_password || ""}
          onChange={(e) => handleChange("proxy_password", e.target.value)}
          placeholder={t("settings.proxyAuthPlaceholder", { defaultValue: "Leave empty if no auth" })}
        />
        <p className="ui-modal-hint">
          {t("settings.proxyHint", { defaultValue: "Proxy settings apply to OpenAI and DeepL translation requests. Saved to config file." })}
        </p>
      </div>

      {/* ESP Mode */}
      <div className="dialog-section">
        <h4>{t("settings.espMode", { defaultValue: "ESP Mode" })}</h4>

        <label className="dialog-label settings-checkbox-label">
          <input
            type="checkbox"
            checked={config.esp_mode ?? false}
            onChange={(e) => setConfig((prev) => ({ ...prev, esp_mode: e.target.checked }))}
          />
          {t("settings.enableEspMode", { defaultValue: "Enable ESP write-back mode" })}
        </label>
        <p className="ui-modal-hint">
          {t("settings.espModeHint", { defaultValue: "When enabled, Save writes translations directly into the ESP file (for delocalized ESPs). When disabled, saves to external .STRINGS files (default)." })}
        </p>
      </div>
    </Modal>
  );
}
