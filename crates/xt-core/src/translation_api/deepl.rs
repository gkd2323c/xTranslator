//! DeepL 翻译服务商
//!
//! 实现用于文本翻译的 DeepL API。
//! 支持免费和专业版 API 端点。
//!
//! DeepL API 预期：
//! - POST https://api-free.deepl.com/v2/translate（免费）或 https://api.deepl.com/v2/translate（专业）
//! - Headers: "Authorization: DeepL-Auth-Key <key>"
//! - Form data: text=<source_text>&target_lang=<target_lang>[&source_lang=<source_lang>]
//! - 返回包含 {detected_source_language, text} 的 translations 数组 JSON

//! DeepL 翻译服务商
//!
//! 支持免费和专业版 DeepL API 端点。
//! 根据 API 密钥是否以 ':fx' 结尾，自动检测免费版与专业版
//! （与 Delphi TESVT_TranslatorApi.pas 行为一致）。
use anyhow::Result;
use serde::Deserialize;

#[derive(Clone)]
pub struct DeepLProvider {
    /// DeepL API 密钥
    api_key: String,
    /// 是否使用免费版 API（true）或专业版 API（false）
    /// 由 api_key 是否以 ":fx" 结尾决定
    use_free_api: bool,
    /// 可选的自定义 API 端点（覆盖免费/专业检测）
    endpoint: Option<String>,
}

impl DeepLProvider {
    /// 从 API 密钥创建一个新的 DeepL 服务商。
    ///
    /// 自动检测是否使用免费或专业版 API：
    /// - 如果密钥以 ":fx" 结尾，使用免费版 API
    /// - 否则，使用专业版 API
    ///
    /// # 参数
    /// * `api_key` - DeepL API 密钥
    ///
    /// # 返回
    /// 新的 DeepLProvider 实例
    pub fn new(api_key: String) -> Self {
        let use_free_api = api_key.ends_with(":fx");
        Self {
            api_key,
            use_free_api,
            endpoint: None,
        }
    }

    /// 设置自定义 API 端点（覆盖免费/专业检测）
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// 获取合适的 DeepL API 端点
    fn get_endpoint(&self) -> String {
        if let Some(ref endpoint) = self.endpoint {
            endpoint.clone()
        } else if self.use_free_api {
            "https://api-free.deepl.com/v2/translate".to_string()
        } else {
            "https://api.deepl.com/v2/translate".to_string()
        }
    }

    /// 获取授权 header 值
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
        proxy: Option<&crate::config::AppConfig>,
    ) -> Result<String> {
        let (protected, crlf_style) = super::protect_crlf(text);
        let client = match proxy {
            Some(cfg) => super::build_client(cfg),
            None => reqwest::Client::new(),
        };
        let url = self.get_endpoint();

        let mut params = vec![("text", protected.as_str()), ("target_lang", target_lang)];

        // 如果指定了源语言且不为空，则添加源语言
        if !source_lang.is_empty() {
            params.push(("source_lang", source_lang));
        }

        // 发送请求
        let response = client
            .post(&url)
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request to DeepL API: {}", e))?;

        // 处理 HTTP 错误
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("DeepL API error ({}): {}", status, body));
        }

        // 解析响应
        let response_json: DeepLResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse DeepL API response: {}", e))?;

        // 提取翻译文本
        response_json
            .translations
            .first()
            .ok_or_else(|| anyhow::anyhow!("No translation in DeepL API response"))
            .map(|t| super::restore_crlf_with_style(&t.text, crlf_style))
    }
}

/// DeepL API 响应结构
#[derive(Debug, Deserialize)]
struct DeepLResponse {
    /// 翻译结果数组
    translations: Vec<DeepLTranslation>,
}

/// 来自 DeepL API 的单个翻译结果
#[derive(Debug, Deserialize)]
struct DeepLTranslation {
    /// 翻译后的文本
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepl_provider_new_free_api() {
        let provider = DeepLProvider::new("test-key:fx".to_string());
        assert_eq!(provider.api_key, "test-key:fx");
        assert!(provider.use_free_api);
        assert_eq!(
            provider.get_endpoint(),
            "https://api-free.deepl.com/v2/translate"
        );
    }

    #[test]
    fn test_deepl_provider_new_pro_api() {
        let provider = DeepLProvider::new("test-key".to_string());
        assert_eq!(provider.api_key, "test-key");
        assert!(!provider.use_free_api);
        assert_eq!(
            provider.get_endpoint(),
            "https://api.deepl.com/v2/translate"
        );
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
