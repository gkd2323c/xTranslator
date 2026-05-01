//! 应用配置持久化

use serde::{Deserialize, Serialize};
use std::path::Path;

const CONFIG_FILENAME: &str = "config.json";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deepl_api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_server: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_password: Option<String>,
    /// ESP mode: when true, save operations write back to the ESP file directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub esp_mode: Option<bool>,
}

impl AppConfig {
    pub fn config_path(dir: &Path) -> std::path::PathBuf {
        dir.join(CONFIG_FILENAME)
    }

    pub fn load(dir: &Path) -> std::io::Result<Self> {
        let path = Self::config_path(dir);
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data).unwrap_or_default())
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = Self::config_path(dir);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)
    }

    pub fn apply(&mut self, other: &Self) {
        if other.openai_api_key.is_some() { self.openai_api_key = other.openai_api_key.clone(); }
        if other.deepl_api_key.is_some() { self.deepl_api_key = other.deepl_api_key.clone(); }
        if other.current_provider.is_some() { self.current_provider = other.current_provider.clone(); }
        if other.theme.is_some() { self.theme = other.theme.clone(); }
        if other.language.is_some() { self.language = other.language.clone(); }
        if other.proxy_server.is_some() { self.proxy_server = other.proxy_server.clone(); }
        if other.proxy_port.is_some() { self.proxy_port = other.proxy_port; }
        if other.proxy_username.is_some() { self.proxy_username = other.proxy_username.clone(); }
        if other.proxy_password.is_some() { self.proxy_password = other.proxy_password.clone(); }
        if other.esp_mode.is_some() { self.esp_mode = other.esp_mode; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default() {
        let dir = std::env::temp_dir().join("xt_config_test_default");
        let _ = std::fs::remove_dir_all(&dir);
        let config = AppConfig::load(&dir).unwrap();
        assert!(config.openai_api_key.is_none());
        assert!(config.theme.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join("xt_config_test_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        let config = AppConfig {
            openai_api_key: Some("sk-test".to_string()),
            theme: Some("dark".to_string()),
            language: Some("zh-CN".to_string()),
            ..Default::default()
        };
        config.save(&dir).unwrap();
        let loaded = AppConfig::load(&dir).unwrap();
        assert_eq!(loaded.openai_api_key.as_deref(), Some("sk-test"));
        assert_eq!(loaded.theme.as_deref(), Some("dark"));
        assert_eq!(loaded.language.as_deref(), Some("zh-CN"));
        assert!(loaded.deepl_api_key.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_skip_none_serialization() {
        let config = AppConfig {
            theme: Some("dark".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("theme"));
        assert!(!json.contains("openai_api_key"));
        assert!(!json.contains("deepl_api_key"));
    }
}
