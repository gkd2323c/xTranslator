//! Microsoft Azure Translator provider
//!
//! Implements the Azure Cognitive Services Translator API v3.0.
//!
//! Auth: Ocp-Apim-Subscription-Key header
//! Endpoint: https://api.cognitive.microsofttranslator.com/translate?api-version=3.0
//! Method: POST with JSON body [{"Text": "..."}]
//! Response: [{"translations": [{"text": "..."}]}]

use anyhow::Result;

#[derive(Clone)]
pub struct AzureProvider {
    subscription_key: String,
}

impl AzureProvider {
    pub fn new(subscription_key: String) -> Self {
        Self { subscription_key }
    }
}

#[async_trait::async_trait]
impl super::TranslationProvider for AzureProvider {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        proxy: Option<&crate::config::AppConfig>,
    ) -> Result<String> {
        let (protected, crlf_style) = super::protect_crlf(text);

        let client = match proxy {
            Some(cfg) => super::build_client(cfg),
            None => reqwest::Client::new(),
        };

        let url = format!(
            "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&from={}&to={}",
            source_lang, target_lang
        );

        let body = serde_json::json!([{"Text": protected}]);

        let response = client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", &self.subscription_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Azure API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Azure API error ({}): {}", status, body));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Azure API invalid JSON: {}", e))?;

        let translated = json[0]["translations"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Azure API: no translations[0].text in response"))?;

        Ok(super::restore_crlf_with_style(translated, crlf_style))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_provider_creation() {
        let provider = AzureProvider::new("test-key".to_string());
        assert_eq!(provider.subscription_key, "test-key");
    }
}
