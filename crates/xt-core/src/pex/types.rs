//! PEX 类型定义

/// PEX 大小端模式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PexEndian {
    LittleEndian,
    BigEndian,
}

/// PEX 文件头（严格对齐 Bethesda 真实 PEX 规范与 Delphi rPexheader）
#[derive(Clone, Debug, PartialEq)]
pub struct PexHeader {
    pub magic: u32,
    pub endian: PexEndian,
    pub major_version: u8,
    pub minor_version: u8,
    pub game_id: u16,
    /// 编译时间（来自头部 timeData）
    pub compile_time: u64,
    /// 源脚本文件名（.psc 文件名）
    pub source_file_name: String,
    /// 编译用户名
    pub user_name: String,
    /// 编译机器名
    pub computer_name: String,
}

impl PexHeader {
    pub fn is_big_endian(&self) -> bool {
        self.endian == PexEndian::BigEndian
    }
}

/// PEX 字符串表中的字符串引用
#[derive(Clone, Debug, PartialEq)]
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
    /// 头部到字符串表前的原始字节（用于重编译完美保留原始头部字节与大小端）
    pub header_raw: Vec<u8>,
    /// 字符串表之后的全部原始字节（包括 DebugInfo、UserFlags、Objects）
    pub data_raw: Vec<u8>,
}
