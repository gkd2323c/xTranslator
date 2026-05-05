//! Youdao translation provider
//!
//! Implements the Youdao Translate API (https://ai.youdao.com).
//!
//! Authentication: MD5(appKey + text + salt + secretKey) signature.
//! Endpoint: http://openapi.youdao.com/api
//! Response JSON field: translation[0]

use anyhow::Result;

#[derive(Clone)]
pub struct YoudaoProvider {
    app_key: String,
    secret_key: String,
}

impl YoudaoProvider {
    pub fn new(app_key: String, secret_key: String) -> Self {
        Self {
            app_key,
            secret_key,
        }
    }

    fn compute_sign(&self, text: &str, salt: &str) -> String {
        let sign_str = format!("{}{}{}{}", self.app_key, text, salt, self.secret_key);
        crate::md5::md5_hex(&sign_str)
    }
}

#[async_trait::async_trait]
impl super::TranslationProvider for YoudaoProvider {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        proxy: Option<&crate::config::AppConfig>,
    ) -> Result<String> {
        let (protected, crlf_style) = super::protect_crlf(text);

        let salt = "1435660288";
        let sign = self.compute_sign(&protected, salt);

        let client = match proxy {
            Some(cfg) => super::build_client(cfg),
            None => reqwest::Client::new(),
        };

        let url = format!(
            "http://openapi.youdao.com/api?appKey={}&q={}&from={}&to={}&sign={}&salt={}",
            self.app_key,
            urlencoding(&protected),
            source_lang,
            target_lang,
            sign,
            salt
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Youdao API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Youdao API error ({}): {}", status, body));
        }

        let body = response.text().await.unwrap_or_default();

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Youdao API invalid JSON: {} — body: {}", e, body))?;

        if let Some(error_code) = json.get("errorCode") {
            let code = error_code.as_str().unwrap_or("");
            if code != "0" {
                return Err(anyhow::anyhow!(
                    "Youdao API error: code={}, body={}",
                    code,
                    body
                ));
            }
        }

        let translated = json["translation"][0]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!("Youdao API: no 'translation' field in response — body: {}", body)
            })?;

        Ok(super::restore_crlf_with_style(translated, crlf_style))
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
    fn test_yooudao_sign() {
        let provider = YoudaoProvider::new(
            "test_app_key".to_string(),
            "test_secret".to_string(),
        );
        let sign = provider.compute_sign("hello", "1435660288");
        assert!(!sign.is_empty());
        assert_eq!(sign.len(), 32);
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello"), "hello");
        assert_eq!(urlencoding("hello world"), "hello%20world");
    }
}
