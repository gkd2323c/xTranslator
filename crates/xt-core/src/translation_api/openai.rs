//! OpenAI 兼容翻译 Provider
//!
//! 支持 OpenAI、DeepSeek、任何兼容 Chat Completions API 的服务。
//! 通过 ApiTranslator.txt 配置模型列表、查询模板、语言映射。

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::config::ApiTranslatorConfig;

/// OpenAI 兼容翻译 Provider
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
    config: Option<ApiTranslatorConfig>,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            config: None,
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_config(mut self, config: ApiTranslatorConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("XT_TRANSLATE_API_KEY")
            .map_err(|_| anyhow!("XT_TRANSLATE_API_KEY environment variable not set"))?;
        let mut provider = Self::new(api_key);
        if let Ok(url) = std::env::var("XT_TRANSLATE_API_BASE") {
            provider = provider.with_base_url(url);
        }
        if let Ok(model) = std::env::var("XT_TRANSLATE_API_MODEL") {
            provider = provider.with_model(model);
        }
        Ok(provider)
    }

    pub fn from_key(api_key: String) -> Self {
        let mut provider = Self::new(api_key);
        if let Ok(url) = std::env::var("XT_TRANSLATE_API_BASE") {
            provider = provider.with_base_url(url);
        }
        if let Ok(model) = std::env::var("XT_TRANSLATE_API_MODEL") {
            provider = provider.with_model(model);
        }
        provider
    }

    fn build_query(&self, text: &str, dest_lang: &str) -> String {
        if let Some(ref cfg) = self.config {
            if let Some(oai) = cfg.get("OpenAI") {
                if let Some(ref template) = oai.default_query {
                    return template.replace("%lang_dest%", dest_lang)
                        + "\n\n"
                        + text;
                }
            }
        }
        // Fallback to built-in prompt
        format!(
            "Translate the following game text to {}. Keep tags and linebreaks exactly as they are:\n\n{}",
            dest_lang, text
        )
    }
}

/// OpenAI Chat Completions 请求体
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI Chat Completions 响应体
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[async_trait]
impl super::TranslationProvider for OpenAIProvider {
    async fn translate(&self, text: &str, _source_lang: &str, target_lang: &str) -> Result<String> {
        let query = self.build_query(text, target_lang);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a professional game translator.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: query,
                },
            ],
            temperature: 0.3,
        };

        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.base_url);

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("API error ({}): {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse API response: {}", e))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .ok_or_else(|| anyhow!("No translation in API response"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env_vars<T>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous: Vec<_> = vars
            .iter()
            .map(|(name, _)| (*name, std::env::var(name).ok()))
            .collect();

        for (name, value) in vars {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        for (name, value) in previous {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }

        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    #[test]
    fn test_openai_provider_from_env_fails_without_key() {
        with_env_vars(&[("XT_TRANSLATE_API_KEY", None)], || {
            assert!(OpenAIProvider::from_env().is_err());
        });
    }

    #[test]
    fn test_openai_provider_from_key() {
        with_env_vars(
            &[
                ("XT_TRANSLATE_API_BASE", None),
                ("XT_TRANSLATE_API_MODEL", None),
            ],
            || {
                let provider = OpenAIProvider::from_key("test-key".to_string());
                assert_eq!(provider.api_key, "test-key");
                assert_eq!(provider.base_url, "https://api.openai.com/v1");
                assert_eq!(provider.model, "gpt-4o-mini");
            },
        );
    }

    #[test]
    fn test_openai_provider_custom_url_and_model() {
        with_env_vars(
            &[
                ("XT_TRANSLATE_API_BASE", Some("https://api.deepseek.com/v1")),
                ("XT_TRANSLATE_API_MODEL", Some("deepseek-chat")),
            ],
            || {
                let provider = OpenAIProvider::from_key("test-key".to_string());
                assert_eq!(provider.base_url, "https://api.deepseek.com/v1");
                assert_eq!(provider.model, "deepseek-chat");
            },
        );
    }
}
