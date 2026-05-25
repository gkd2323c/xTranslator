//! PEX 类型定义

/// PEX 文件头（在 magic 0xFA57C0DE 之后开始）
#[derive(Clone, Debug)]
pub struct PexHeader {
    pub major_version: u8,
    pub minor_version: u8,
    pub game_id: u16,
    /// 编译时间（来自调试信息部分的 mod_time）
    pub compile_time: u64,
}

/// PEX 字符串表中的字符串引用
#[derive(Clone, Debug)]
pub struct PexStringEntry {
    pub index: u16,
    pub text: String,
}

/// 提取的待翻译字符串
#[derive(Clone, Debug, PartialEq)]
pub struct PexTranslatableString {
    /// 包含该字符串的脚本对象名称
    pub object_name: String,
    /// 状态名称（默认状态为空）
    pub state_name: String,
    /// 函数名称（对象级文档为空）
    pub function_name: String,
    /// 字符串类型："DebugString", "PropertyName" 或 "StringLiteral"
    pub string_type: String,
    /// 要翻译的原始文本
    pub source_text: String,
    /// 翻译后的文本（如果尚未翻译则为空）
    pub translation: String,
}

/// 解析后的 PEX 脚本信息
#[derive(Clone, Debug)]
pub struct PexScript {
    pub header: PexHeader,
    /// 完整的字符串表（索引 -> 文本）
    pub string_table: Vec<PexStringEntry>,
    /// 所有提取的待翻译字符串
    pub translatable: Vec<PexTranslatableString>,
    /// 原始调试信息部分字节（用于在重编译期间保留）
    pub debug_info_raw: Vec<u8>,
    /// 原始用户标志部分字节（用于在重编译期间保留）
    pub user_flags_raw: Vec<u8>,
    /// 每个对象的原始对象体字节（用于在重编译期间保留）
    pub object_bodies_raw: Vec<Vec<u8>>,
}
