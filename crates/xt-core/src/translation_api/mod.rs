//! 翻译 API 抽象层
//!
//! 支持多种翻译 provider：
//! - OpenAI 兼容 API（OpenAI / DeepSeek / 通义千问 / 等）
//! - DeepL API（免费版 / 专业版）

use anyhow::Result;
use std::fmt;

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

/// OpenAI 兼容翻译 Provider
pub mod openai;
pub use openai::OpenAIProvider;

/// DeepL 翻译 Provider
pub mod deepl;
pub use deepl::DeepLProvider;
