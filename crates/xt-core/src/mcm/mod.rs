//! MCM (Mod Configuration Menu) 翻译支持
//!
//! MCM 翻译文件格式：
//! - 位置：`interface\translations\<ModName>_lang.txt`
//! - 编码：UTF-16LE（Delphi tEncoding.unicode），回退 UTF-8
//! - 格式：Tab 分隔的键值对，`^(\$.+?)\t+(.+)$`
//!   - 键（Group 1）：`$sMySetting` 形式的标识符
//!   - 值（Group 2）：可翻译字符串
//!
//! 归一化策略（与 Delphi 一致）：
//! - 解析时：将原文替换为 `{{xt=N}}` 占位符，N 为字符串在列表中的索引
//! - 保存时：反向替换，将翻译填回原位置

pub mod parser;
pub mod types;

pub use parser::{parse_mcm_file, save_mcm_file};
pub use types::{McmEntry, McmFile};