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

/// 翻译前：将换行符替换为保护标签
pub fn protect_crlf(text: &str) -> String {
    text.replace("\r\n", CRLF_TAG)
        .replace('\r', "")
        .replace('\n', CRLF_TAG)
}

/// 翻译后：将保护标签还原为换行符
pub fn restore_crlf(text: &str) -> String {
    text.replace(CRLF_TAG, "\r\n")
}

/// Build an optional reqwest::Proxy from AppConfig proxy settings
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

/// Build a reqwest::Client with optional proxy
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
        assert_eq!(protect_crlf("line1\nline2"), "line1<L_F>line2");
    }

    #[test]
    fn test_protect_crlf_crlf() {
        assert_eq!(protect_crlf("line1\r\nline2"), "line1<L_F>line2");
    }

    #[test]
    fn test_protect_crlf_multiline() {
        assert_eq!(protect_crlf("a\n\nb"), "a<L_F><L_F>b");
    }

    #[test]
    fn test_restore_crlf() {
        assert_eq!(restore_crlf("line1<L_F>line2"), "line1\r\nline2");
    }

    #[test]
    fn test_roundtrip() {
        let orig = "Hello\r\nWorld\nTest\r\nFoo";
        let protected = protect_crlf(orig);
        let restored = restore_crlf(&protected);
        // Note: \n alone becomes <L_F> which restores to \r\n
        // So roundtrip normalizes \n to \r\n — acceptable behavior
        assert_eq!(restored, "Hello\r\nWorld\r\nTest\r\nFoo");
    }

    #[test]
    fn test_no_newlines_unchanged() {
        assert_eq!(protect_crlf("Hello World"), "Hello World");
        assert_eq!(restore_crlf("Hello World"), "Hello World");
    }
}

/// 翻译 Provider 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI 兼容 API（默认）
    OpenAI,
    /// DeepL API（自动检测免费版/专业版）
    DeepL,
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::DeepL => write!(f, "deepl"),
        }
    }
}

impl ProviderType {
    /// 从字符串解析 Provider 类型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "deepl" => ProviderType::DeepL,
            _ => ProviderType::OpenAI,
        }
    }

    /// 获取所有可用 Provider 列表
    pub fn all() -> Vec<&'static str> {
        vec!["openai", "deepl"]
    }
}

/// 翻译 Provider trait
#[async_trait::async_trait]
pub trait TranslationProvider: Send + Sync {
    /// 翻译文本（异步）
    async fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String>;
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
