//! DeepL translation provider
//!
//! Implements the DeepL API for text translation.
//! Supports both free and pro API endpoints.
//! 
//! The DeepL API expects:
//! - POST https://api-free.deepl.com/v2/translate (free) or https://api.deepl.com/v2/translate (pro)
//! - Headers: "Authorization: DeepL-Auth-Key <key>"
//! - Form data: text=<source_text>&target_lang=<target_lang>[&source_lang=<source_lang>]
//! - Returns JSON with translations array containing {detected_source_language, text}

//! DeepL translation provider
//!
//! Supports both free and pro DeepL API endpoints.
//! Automatically detects free vs pro based on API key ending with ':fx'
//! (matching Delphi TESVT_TranslatorApi.pas behavior).
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Clone)]
pub struct DeepLProvider {
    /// DeepL API key
    api_key: String,
    /// Whether to use the free API (true) or pro API (false)
    /// Determined by whether the api_key ends with ":fx"
    use_free_api: bool,
    /// Optional custom API endpoint (overrides free/pro detection)
    endpoint: Option<String>,
}

impl DeepLProvider {
    /// Create a new DeepL provider from an API key.
    ///
    /// Automatically detects whether to use free or pro API:
    /// - If key ends with ":fx", uses free API
    /// - Otherwise, uses pro API
    ///
    /// # Arguments
    /// * `api_key` - DeepL API key
    ///
    /// # Returns
    /// New DeepLProvider instance
    pub fn new(api_key: String) -> Self {
        let use_free_api = api_key.ends_with(":fx");
        Self {
            api_key,
            use_free_api,
            endpoint: None,
        }
    }

    /// Set a custom API endpoint (overrides free/pro detection)
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// Get the appropriate DeepL API endpoint
    fn get_endpoint(&self) -> String {
        if let Some(ref endpoint) = self.endpoint {
            endpoint.clone()
        } else if self.use_free_api {
            "https://api-free.deepl.com/v2/translate".to_string()
        } else {
            "https://api.deepl.com/v2/translate".to_string()
        }
    }

    /// Get the authorization header value
    fn get_auth_header(&self) -> String {
        format!("DeepL-Auth-Key {}", self.api_key)
    }
}

#[async_trait::async_trait]
impl super::TranslationProvider for DeepLProvider {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let client = reqwest::Client::new();
        let url = self.get_endpoint();

        // Build request parameters
        let mut params = vec![
            ("text", text),
            ("target_lang", target_lang),
        ];

        // Add source language if specified and not empty
        if !source_lang.is_empty() {
            params.push(("source_lang", source_lang));
        }

        // Send request
        let response = client
            .post(&url)
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request to DeepL API: {}", e))?;

        // Handle HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("DeepL API error ({}): {}", status, body));
        }

        // Parse response
        let response_json: DeepLResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse DeepL API response: {}", e))?;

        // Extract translated text
        response_json
            .translations
            .first()
            .ok_or_else(|| anyhow::anyhow!("No translation in DeepL API response"))
            .map(|t| t.text.clone())
    }
}

/// DeepL API response structure
#[derive(Debug, Deserialize)]
struct DeepLResponse {
    /// Array of translation results
    translations: Vec<DeepLTranslation>,
}

/// Individual translation result from DeepL API
#[derive(Debug, Deserialize)]
struct DeepLTranslation {
    /// Detected source language (may differ from requested source_lang)
    #[serde(rename = "detected_source_language")]
    detected_source_language: String,
    /// Translated text
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_deepl_provider_new_free_api() {
        let provider = DeepLProvider::new("test-key:fx".to_string());
        assert_eq!(provider.api_key, "test-key:fx");
        assert!(provider.use_free_api);
        assert_eq!(provider.get_endpoint(), "https://api-free.deepl.com/v2/translate");
    }

    #[test]
    fn test_deepl_provider_new_pro_api() {
        let provider = DeepLProvider::new("test-key".to_string());
        assert_eq!(provider.api_key, "test-key");
        assert!(!provider.use_free_api);
        assert_eq!(provider.get_endpoint(), "https://api.deepl.com/v2/translate");
    }

    #[test]
    fn test_deepl_provider_with_endpoint() {
        let provider = DeepLProvider::new("test-key".to_string())
            .with_endpoint("https://custom.deepl.com/v2/translate".to_string());
        assert_eq!(
            provider.get_endpoint(),
            "https://custom.deepl.com/v2/translate"
        );
    }
}