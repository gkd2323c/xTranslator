//! PEX (Papyrus Executable) 脚本二进制格式解析器
//!
//! Bethesda 的 Papyrus 脚本语言将源脚本 (.psc) 编译为
//! 编译后的二进制 .pex 文件。此模块解析 PEX 文件并提取
//! 待翻译字符串：DebugString（文档）、PropertyName、StringLiteral。
//!
//! 基于 Delphi TESVT_scriptPex.pas 和社区格式文档。
//!
//! 范围：字符串提取 + v1.5 写回支持（字符串表重建）。

pub mod compile;
pub mod decompile;
pub mod parser;
pub mod types;
