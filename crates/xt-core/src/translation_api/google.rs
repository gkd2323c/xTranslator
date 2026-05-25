//! Google Translate 翻译服务商
//!
//! 使用公开（无密钥）的 Google Translate API。
//! Endpoint: https://translate.googleapis.com/translate_a/single
//!
//! Response 格式: [[["translated_text", "original", null, null, ...], ...], null, "src_lang"]

use anyhow::Result;

#[derive(Clone, Default)]
pub struct GoogleProvider;

impl GoogleProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl super::TranslationProvider for GoogleProvider {
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
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            source_lang,
            target_lang,
            urlencoding(&protected)
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Google API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Google API error ({}): {}", status, body));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Google API invalid JSON: {}", e))?;

        // Google 响应: [[["text", "orig", ...], ...], null, "src"]
        let segments = json[0]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Google API: unexpected response format"))?;

        let mut result = String::new();
        for seg in segments {
            if let Some(text) = seg[0].as_str() {
                result.push_str(text);
            }
        }

        if result.is_empty() {
            return Err(anyhow::anyhow!("Google API: empty translation result"));
        }

        Ok(super::restore_crlf_with_style(&result, crlf_style))
    }
}

fn urlencoding(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 3);
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_provider_creation() {
        let provider = GoogleProvider::new();
        let _ = provider;
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello%20world");
    }
}
