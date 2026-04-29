//! API 翻译器配置 — 解析 Delphi 原版 `ApiTranslator.txt`
//!
//! 文件格式：`Provider_Key=Value`，按 Provider 前缀分组。
//! 支持的配置项：
//! - `enabled` — 是否启用
//! - `Label` — 显示名称
//! - `CharLimit` — 单次查询最大字符数
//! - `ArrayLimit` — 批量请求最大条目数
//! - `ArrayTimePause` — 批量请求间隔秒数
//! - `ArrayMaxCharPerMin` — 每分钟最大字符数
//! - `SingleTimePause` — 单次请求间隔秒数
//! - `ApiUrl` — API 端点 URL
//! - `ProApiUrl` — DeepL 专业版端点
//! - `DefaultQuery` — OpenAI 默认翻译提示
//! - `Model0..9` — OpenAI 模型列表
//! - `{lang}` — 语言代码映射

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ApiProviderConfig {
    pub enabled: bool,
    pub label: String,
    pub char_limit: u32,
    pub array_limit: u32,
    pub array_pause_secs: u32,
    pub max_char_per_min: u32,
    pub single_pause_secs: u32,
    pub api_url: Option<String>,
    pub pro_api_url: Option<String>,
    pub default_query: Option<String>,
    pub models: Vec<String>,
    pub lang_codes: HashMap<String, String>, // key -> API-specific lang code
}

#[derive(Clone, Debug, Default)]
pub struct ApiTranslatorConfig {
    pub providers: HashMap<String, ApiProviderConfig>,
}

impl ApiTranslatorConfig {
    pub fn load_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::parse(&content))
    }

    pub fn parse(content: &str) -> Self {
        let mut raw: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim().to_string();
                let value = line[eq + 1..].trim().to_string();
                raw.insert(key, value);
            }
        }

        let mut providers: HashMap<String, ApiProviderConfig> = HashMap::new();

        // Collect all unique provider prefixes
        let mut prefixes: Vec<&str> = Vec::new();
        for key in raw.keys() {
            if let Some(us) = key.find('_') {
                let prefix = &key[..us];
                if !prefixes.contains(&prefix) {
                    prefixes.push(prefix);
                }
            }
        }

        for prefix in &prefixes {
            let mut cfg = ApiProviderConfig::default();

            cfg.enabled = raw.get(&format!("{}_enabled", prefix))
                .map(|v| v == "true")
                .unwrap_or(false);

            cfg.label = raw.get(&format!("{}_Label", prefix))
                .cloned()
                .unwrap_or_else(|| prefix.to_string());

            cfg.char_limit = raw.get(&format!("{}_CharLimit", prefix))
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000);

            cfg.array_limit = raw.get(&format!("{}_ArrayLimit", prefix))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            cfg.array_pause_secs = raw.get(&format!("{}_ArrayTimePause", prefix))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            cfg.max_char_per_min = raw.get(&format!("{}_ArrayMaxCharPerMin", prefix))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            cfg.single_pause_secs = raw.get(&format!("{}_SingleTimePause", prefix))
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);

            cfg.api_url = raw.get(&format!("{}_ApiUrl", prefix)).cloned();
            cfg.pro_api_url = raw.get(&format!("{}_ProApiUrl", prefix)).cloned();
            cfg.default_query = raw.get(&format!("{}_DefaultQuery", prefix)).cloned();

            // Models: OpenAI_Model0..9
            for i in 0..10u8 {
                let model_key = format!("{}_Model{}", prefix, i);
                if let Some(model) = raw.get(&model_key) {
                    cfg.models.push(model.clone());
                }
            }

            // Language codes: {prefix}_{lang}=code
            let prefix_underscore = format!("{}_", prefix);
            for (key, value) in &raw {
                if key.starts_with(&prefix_underscore) {
                    let suffix = &key[prefix_underscore.len()..];
                    // Skip known config keys
                    if matches!(
                        suffix,
                        "enabled" | "Label" | "CharLimit" | "ArrayLimit"
                            | "ArrayTimePause" | "ArrayMaxCharPerMin"
                            | "SingleTimePause" | "ApiUrl" | "ProApiUrl"
                            | "DefaultQuery" | "Powered" | "PoweredUrl"
                    ) {
                        continue;
                    }
                    // Skip model keys
                    if suffix.starts_with("Model") && suffix.len() > 5 {
                        continue;
                    }
                    cfg.lang_codes.insert(suffix.to_string(), value.clone());
                }
            }

            providers.insert(prefix.to_string(), cfg);
        }

        Self { providers }
    }

    pub fn get(&self, provider: &str) -> Option<&ApiProviderConfig> {
        self.providers.get(provider)
    }

    /// Resolve a language name to the API-specific language code
    pub fn resolve_lang(&self, provider: &str, lang: &str) -> String {
        if let Some(cfg) = self.get(provider) {
            // Try exact match first (e.g. "english")
            if let Some(code) = cfg.lang_codes.get(lang) {
                return code.clone();
            }
            // Try short form (e.g. "en")
            let short = lang_to_short(lang);
            if let Some(code) = cfg.lang_codes.get(&short) {
                return code.clone();
            }
            // Try simplified chinese variants
            if lang == "chinese" || lang == "cn" {
                if let Some(code) = cfg.lang_codes.get("zhhans") {
                    return code.clone();
                }
            }
        }
        // Fallback: use lang as-is
        lang.to_string()
    }
}

fn lang_to_short(lang: &str) -> String {
    match lang.to_lowercase().as_str() {
        "english" => "en",
        "french" => "fr",
        "german" => "de",
        "italian" => "it",
        "spanish" => "es",
        "portuguese" => "ptbr",
        "polish" => "pl",
        "russian" => "ru",
        "japanese" => "ja",
        "chinese" => "cn",
        "korean" => "ko",
        "czech" => "cs",
        "danish" => "da",
        "finnish" => "fi",
        "greek" => "el",
        "norwegian" => "no",
        "swedish" => "sv",
        "turkish" => "tr",
        "hungarian" => "hu",
        "arabic" => "ar",
        "estonian" => "et",
        "ukrainian" => "uk",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openai() {
        let content = r#"
OpenAI_enabled=true
OpenAI_Label=OpenAI/ChatGpt
OpenAI_CharLimit=7500
OpenAI_ArrayLimit=49
OpenAI_ApiUrl=https://api.openai.com/v1/chat/completions
OpenAI_DefaultQuery=Translate the following text to %lang_dest%
OpenAI_Model0=gpt-3.5-turbo
OpenAI_Model1=gpt-4
OpenAI_english=english
OpenAI_french=french
OpenAI_cn=chinese
OpenAI_ja=japanese
"#;
        let config = ApiTranslatorConfig::parse(content);
        let oai = config.get("OpenAI").unwrap();
        assert!(oai.enabled);
        assert_eq!(oai.label, "OpenAI/ChatGpt");
        assert_eq!(oai.char_limit, 7500);
        assert_eq!(oai.array_limit, 49);
        assert_eq!(oai.models, vec!["gpt-3.5-turbo", "gpt-4"]);
        assert_eq!(oai.lang_codes.get("english").unwrap(), "english");
        assert_eq!(oai.lang_codes.get("cn").unwrap(), "chinese");
    }

    #[test]
    fn test_parse_deepl() {
        let content = r#"
DeepL_enabled=true
DeepL_Label=DeepL
DeepL_CharLimit=9000
DeepL_ArrayLimit=49
DeepL_ApiUrl=https://api-free.deepl.com/v2/translate
DeepL_ProApiUrl=https://api.deepl.com/v2/translate
DeepL_english=en
DeepL_russian=ru
"#;
        let config = ApiTranslatorConfig::parse(content);
        let dl = config.get("DeepL").unwrap();
        assert!(dl.enabled);
        assert_eq!(dl.api_url.as_deref(), Some("https://api-free.deepl.com/v2/translate"));
        assert_eq!(dl.pro_api_url.as_deref(), Some("https://api.deepl.com/v2/translate"));
        assert_eq!(dl.lang_codes.get("english").unwrap(), "en");
    }

    #[test]
    fn test_resolve_lang() {
        let content = "DeepL_english=en\nDeepL_cn=zh\nDeepL_french=fr\n";
        let config = ApiTranslatorConfig::parse(content);
        assert_eq!(config.resolve_lang("DeepL", "english"), "en");
        assert_eq!(config.resolve_lang("DeepL", "french"), "fr");
        // Fallback
        assert_eq!(config.resolve_lang("DeepL", "unknown"), "unknown");
    }

    #[test]
    fn test_disabled_provider() {
        let content = "Google_enabled=false\nGoogle_Label=Google\n";
        let config = ApiTranslatorConfig::parse(content);
        assert!(!config.get("Google").unwrap().enabled);
    }
}
