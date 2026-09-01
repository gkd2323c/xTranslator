//! PEX 类型定义 — 严格对齐 Bethesda 真实 PEX 规范与 Delphi `TESVT_scriptPex.pas` 格式。

/// PEX 大小端模式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PexEndian {
    LittleEndian,
    BigEndian,
}

/// PEX 文件头（严格对齐真实 PEX 格式：Magic + Major/Minor + GameID + CompilationTime + Source/User/Computer）
#[derive(Clone, Debug, PartialEq)]
pub struct PexHeader {
    pub magic: u32,
    pub endian: PexEndian,
    pub major_version: u8,
    pub minor_version: u8,
    pub game_id: u16,
    pub compile_time: u64,
    pub source_file_name: String,
    pub user_name: String,
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
    pub object_name: String,
    pub state_name: String,
    pub function_name: String,
    pub string_type: String,
    pub source_text: String,
    pub translation: String,
}

/// 解析后的 PEX 脚本信息
/// - header_raw: 从 magic 到 stringTableCount (u16) 的完整原始字节，用于重编译时完美保留
/// - data_raw: stringTable 之后的全部原始字节 (hasDebugInfo + debugInfo + userFlags + objects)
#[derive(Clone, Debug)]
pub struct PexScript {
    pub header: PexHeader,
    pub string_table: Vec<PexStringEntry>,
    pub translatable: Vec<PexTranslatableString>,
    pub header_raw: Vec<u8>,
    pub data_raw: Vec<u8>,
}