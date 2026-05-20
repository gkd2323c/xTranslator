//! Baidu translation provider
//!
//! Implements the Baidu Translate API (https://fanyi-api.baidu.com).
//!
//! Authentication: MD5(appId + text + salt + key) signature.
//! Endpoint: http://api.fanyi.baidu.com/api/trans/vip/translate
//! Response JSON field: dst

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct BaiduProvider {
    app_id: String,
    key: String,
}

impl BaiduProvider {
    pub fn new(app_id: String, key: String) -> Self {
        Self { app_id, key }
    }

    fn compute_sign(&self, text: &str, salt: &str) -> String {
        let sign_str = format!("{}{}{}{}", self.app_id, text, salt, self.key);
        crate::md5::md5_hex(&sign_str)
    }
}

#[async_trait::async_trait]
impl super::TranslationProvider for BaiduProvider {
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        proxy: Option<&crate::config::AppConfig>,
    ) -> Result<String> {
        let (protected, crlf_style) = super::protect_crlf(text);

        // 每次请求生成随机 salt（时间戳纳秒级确保唯一性）
        let salt = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string();
        let sign = self.compute_sign(&protected, &salt);

        let client = match proxy {
            Some(cfg) => super::build_client(cfg),
            None => reqwest::Client::new(),
        };

        let url = format!(
            "http://api.fanyi.baidu.com/api/trans/vip/translate?appid={}&q={}&from={}&to={}&salt={}&sign={}",
            self.app_id,
            urlencoding(&protected),
            source_lang,
            target_lang,
            salt,
            sign
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Baidu API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Baidu API error ({}): {}", status, body));
        }

        let body = response.text().await.unwrap_or_default();

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Baidu API invalid JSON: {} — body: {}", e, body))?;

        if let Some(error_code) = json.get("error_code") {
            let error_msg = json.get("error_msg").and_then(|v| v.as_str()).unwrap_or("unknown");
            return Err(anyhow::anyhow!(
                "Baidu API error: code={}, msg={}",
                error_code,
                error_msg
            ));
        }

        let translated = json["trans_result"][0]["dst"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Baidu API: no 'dst' field in response — body: {}", body))?;

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
    fn test_baidu_sign() {
        let provider = BaiduProvider::new(
            "20200101000399999".to_string(),
            "abcdefg".to_string(),
        );
        let sign = provider.compute_sign("apple", "1435660288");
        assert!(!sign.is_empty());
        assert_eq!(sign.len(), 32); // MD5 hex
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello"), "hello");
        assert_eq!(urlencoding("hello world"), "hello%20world");
        assert_eq!(urlencoding("你好"), "%E4%BD%A0%E5%A5%BD");
    }
}
