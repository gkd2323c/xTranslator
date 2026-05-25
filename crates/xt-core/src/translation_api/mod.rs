//! 翻译 API 抽象层
//!
//! 支持多种翻译 provider：
//! - OpenAI 兼容 API（OpenAI / DeepSeek / 通义千问 / 等）
//! - DeepL API（免费版 / 专业版）
//!
//! 多行文本保护：翻译前将 `\r\n` 替换为 `<L_F>` 标签，
//! 翻译后还原。防止 API 吞掉换行符（与 Delphi 原版一致）。

use anyhow::Result;
use std::fmt;

/// CRLF 保护标签（与 Delphi `CRLFtag = '<L_F>'` 一致）
const CRLF_TAG: &str = "<L_F>";

/// 文本中的换行风格（Delphi 保留原文换行风格）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrlfStyle {
    /// `\r\n` (Windows)
    CrLf,
    /// `\n` (Unix/Linux/macOS)
    Lf,
}

impl CrlfStyle {
    fn as_str(&self) -> &'static str {
        match self {
            CrlfStyle::CrLf => "\r\n",
            CrlfStyle::Lf => "\n",
        }
    }
}

/// 检测文本的主导航行风格
fn detect_crlf_style(text: &str) -> CrlfStyle {
    let has_crlf = text.contains("\r\n");
    let has_lf = text.contains('\n');

    if has_crlf {
        CrlfStyle::CrLf
    } else if has_lf {
        CrlfStyle::Lf
    } else {
        // 没有换行符，默认 Windows 风格
        CrlfStyle::CrLf
    }
}

/// 翻译前：将换行符替换为保护标签，同时记录原始换行风格
///
/// 返回 `(受保护的文本, 原始换行风格)`
pub fn protect_crlf(text: &str) -> (String, CrlfStyle) {
    let style = detect_crlf_style(text);
    let protected = text
        .replace("\r\n", CRLF_TAG)
        .replace('\r', "")
        .replace('\n', CRLF_TAG);
    (protected, style)
}

/// 翻译后：将保护标签还原为换行符（默认 `\r\n`，向后兼容）
pub fn restore_crlf(text: &str) -> String {
    text.replace(CRLF_TAG, "\r\n")
}

/// 翻译后：将保护标签还原为指定风格的换行符
pub fn restore_crlf_with_style(text: &str, style: CrlfStyle) -> String {
    text.replace(CRLF_TAG, style.as_str())
}

/// 从 AppConfig 代理设置构建可选的 reqwest::Proxy
pub fn build_proxy(config: &crate::config::AppConfig) -> Option<reqwest::Proxy> {
    let server = config.proxy_server.as_ref()?;
    let port = config.proxy_port.unwrap_or(8080);
    let proxy_url = if server.contains("://") {
        format!("{}:{}", server, port)
    } else {
        format!("http://{}:{}", server, port)
    };
    let mut proxy = reqwest::Proxy::all(&proxy_url).ok()?;
    if let (Some(user), Some(pass)) = (&config.proxy_username, &config.proxy_password) {
        proxy = proxy.basic_auth(user, pass);
    }
    Some(proxy)
}

/// 构建带可选代理的 reqwest::Client
pub fn build_client(config: &crate::config::AppConfig) -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Some(proxy) = build_proxy(config) {
        builder = builder.proxy(proxy);
    }
    builder.build().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protect_crlf_single_newline() {
        let (protected, style) = protect_crlf("line1\nline2");
        assert_eq!(protected, "line1<L_F>line2");
        assert_eq!(style, CrlfStyle::Lf);
    }

    #[test]
    fn test_protect_crlf_crlf() {
        let (protected, style) = protect_crlf("line1\r\nline2");
        assert_eq!(protected, "line1<L_F>line2");
        assert_eq!(style, CrlfStyle::CrLf);
    }

    #[test]
    fn test_protect_crlf_multiline() {
        let (protected, _) = protect_crlf("a\n\nb");
        assert_eq!(protected, "a<L_F><L_F>b");
    }

    #[test]
    fn test_restore_crlf() {
        assert_eq!(restore_crlf("line1<L_F>line2"), "line1\r\nline2");
    }

    #[test]
    fn test_restore_crlf_with_style_lf() {
        assert_eq!(
            restore_crlf_with_style("line1<L_F>line2", CrlfStyle::Lf),
            "line1\nline2"
        );
    }

    #[test]
    fn test_roundtrip_style_preservation() {
        // Unix 风格换行符应在往返后保持不变
        let orig = "Hello\nWorld\nTest";
        let (protected, style) = protect_crlf(orig);
        assert_eq!(style, CrlfStyle::Lf);
        let restored = restore_crlf_with_style(&protected, style);
        assert_eq!(restored, orig);

        // Windows 风格换行符应在往返后保持不变
        let orig = "Hello\r\nWorld\r\nTest";
        let (protected, style) = protect_crlf(orig);
        assert_eq!(style, CrlfStyle::CrLf);
        let restored = restore_crlf_with_style(&protected, style);
        assert_eq!(restored, orig);
    }

    #[test]
    fn test_no_newlines_unchanged() {
        let (protected, _) = protect_crlf("Hello World");
        assert_eq!(protected, "Hello World");
        assert_eq!(restore_crlf("Hello World"), "Hello World");
    }

    #[test]
    fn test_build_proxy_with_auth() {
        use crate::config::AppConfig;
        let config = AppConfig {
            proxy_server: Some("proxy.example.com".to_string()),
            proxy_port: Some(8080),
            proxy_username: Some("user".to_string()),
            proxy_password: Some("pass".to_string()),
            ..Default::default()
        };
        let proxy = build_proxy(&config);
        assert!(proxy.is_some());
    }

    #[test]
    fn test_build_proxy_without_auth() {
        use crate::config::AppConfig;
        let config = AppConfig {
            proxy_server: Some("proxy.example.com".to_string()),
            proxy_port: Some(8080),
            ..Default::default()
        };
        let proxy = build_proxy(&config);
        assert!(proxy.is_some());
    }

    #[test]
    fn test_build_proxy_no_server() {
        use crate::config::AppConfig;
        let config = AppConfig::default();
        let proxy = build_proxy(&config);
        assert!(proxy.is_none());
    }

    #[test]
    fn test_build_client_with_proxy() {
        use crate::config::AppConfig;
        let config = AppConfig {
            proxy_server: Some("proxy.example.com".to_string()),
            proxy_port: Some(8080),
            proxy_username: Some("user".to_string()),
            proxy_password: Some("pass".to_string()),
            ..Default::default()
        };
        let _client = build_client(&config);
        // 仅验证构建不会崩溃
        assert!(true);
    }
}

/// 翻译 Provider 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI 兼容 API（默认）
    OpenAI,
    /// DeepL API（自动检测免费版/专业版）
    DeepL,
    /// 百度翻译 API
    Baidu,
    /// 有道翻译 API
    Youdao,
    /// 微软 Azure 翻译 API
    Azure,
    /// Google 翻译 API
    Google,
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::DeepL => write!(f, "deepl"),
            ProviderType::Baidu => write!(f, "baidu"),
            ProviderType::Youdao => write!(f, "youdao"),
            ProviderType::Azure => write!(f, "azure"),
            ProviderType::Google => write!(f, "google"),
        }
    }
}

impl ProviderType {
    /// 从字符串解析 Provider 类型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "deepl" => ProviderType::DeepL,
            "baidu" => ProviderType::Baidu,
            "youdao" => ProviderType::Youdao,
            "azure" => ProviderType::Azure,
            "google" => ProviderType::Google,
            _ => ProviderType::OpenAI,
        }
    }

    /// 获取所有可用 Provider 列表
    pub fn all() -> Vec<&'static str> {
        vec!["openai", "deepl", "baidu", "youdao", "azure", "google"]
    }
}

/// 翻译 Provider trait
#[async_trait::async_trait]
pub trait TranslationProvider: Send + Sync {
    /// 翻译文本（异步）
    /// `proxy` — 包含代理设置的可选 AppConfig
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        proxy: Option<&crate::config::AppConfig>,
    ) -> Result<String>;
}

/// API 翻译器配置（解析 ApiTranslator.txt）
pub mod config;
pub use config::{ApiProviderConfig, ApiTranslatorConfig};

/// OpenAI 兼容翻译 Provider
pub mod openai;
pub use openai::OpenAIProvider;

/// DeepL 翻译 Provider
pub mod deepl;
pub use deepl::DeepLProvider;

/// 百度翻译 Provider
pub mod baidu;
pub use baidu::BaiduProvider;

/// 有道翻译 Provider
pub mod youdao;
pub use youdao::YoudaoProvider;

/// 微软 Azure 翻译 Provider
pub mod azure;
pub use azure::AzureProvider;

/// Google 翻译 Provider
pub mod google;
pub use google::GoogleProvider;
