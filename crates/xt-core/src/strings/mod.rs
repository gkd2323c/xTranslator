//! Strings 文件解析模块
//!
//! Skyrim 等 Bethesda 游戏的字符串存储在独立的字符串文件中：
//! - .STRINGS (listIndex=0): null 终止字符串格式
//! - .DLSTRINGS (listIndex=1): 4字节长度前缀 + (length-1)字节内容  
//! - .ILSTRINGS (listIndex=2): 4字节长度前缀 + (length-1)字节内容
//!
//! 编码处理：
//! - 默认使用 UTF-8 编码
//! - 解码失败时可通过 CodepageConfig 回退到 Windows codepage
//! - 支持 codepage.txt 配置文件（对应 Delphi 的 codepage 系统）

pub mod codepage;

pub use codepage::{CodepageConfig, CodepageId, CodepageTable};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StringsFormat {
    NullTerminated,
    LengthPrefixed,
}

pub struct StringsFile {
    pub strings: HashMap<u32, String>,
    pub format: StringsFormat,
    pub codepage: CodepageConfig,
}

impl StringsFile {
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            format: StringsFormat::NullTerminated,
            codepage: CodepageConfig::utf8(),
        }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path_ref = path.as_ref();
        let format = Self::detect_format(path_ref);
        let codepage = CodepageConfig::utf8();
        Self::load_with_format_and_codepage(path, format, codepage)
    }

    pub fn load_with_format<P: AsRef<Path>>(
        path: P,
        format: StringsFormat,
    ) -> std::io::Result<Self> {
        let codepage = CodepageConfig::utf8();
        Self::load_with_format_and_codepage(path, format, codepage)
    }

    /// 加载 Strings 文件，使用指定的 codepage 配置
    ///
    /// 当已知文件编码时使用此方法（例如明确知道是 CP936）
    pub fn load_with_codepage<P: AsRef<Path>>(
        path: P,
        codepage: CodepageConfig,
    ) -> std::io::Result<Self> {
        let path_ref = path.as_ref();
        let format = Self::detect_format(path_ref);
        Self::load_with_format_and_codepage(path, format, codepage)
    }

    /// 加载 Strings 文件，使用 codepage 配置表自动推断编码
    ///
    /// 根据文件名匹配 codepage.txt 中的配置，自动选择正确的编码
    /// 例如：skyrim_english.strings → UTF-8，skyrim_chinese.strings → CP936
    pub fn load_with_codepage_table<P: AsRef<Path>>(
        path: P,
        table: &CodepageTable,
    ) -> std::io::Result<Self> {
        let path_ref = path.as_ref();
        let format = Self::detect_format(path_ref);
        let codepage = table.get_for_filename(&path_ref.to_string_lossy());
        Self::load_with_format_and_codepage(path, format, codepage)
    }

    fn load_with_format_and_codepage<P: AsRef<Path>>(
        path: P,
        format: StringsFormat,
        codepage: CodepageConfig,
    ) -> std::io::Result<Self> {
        let file = File::open(path.as_ref())?;
        let mut reader = std::io::BufReader::new(file);
        Self::load_from_reader(&mut reader, format, codepage)
    }

    /// 从字节缓冲区加载 Strings 文件（用于 BSA 提取等场景）
    pub fn load_from_bytes(
        bytes: &[u8],
        format: StringsFormat,
        codepage: CodepageConfig,
    ) -> std::io::Result<Self> {
        let mut cursor = std::io::Cursor::new(bytes);
        Self::load_from_reader(&mut cursor, format, codepage)
    }

    fn load_from_reader<R: Read + Seek>(
        reader: &mut R,
        format: StringsFormat,
        codepage: CodepageConfig,
    ) -> std::io::Result<Self> {
        // 文件头：
        // - u32 count：字符串条目数量
        // - u32 data_size：数据区总字节数
        let count = reader.read_u32::<LittleEndian>()?;
        let data_size = reader.read_u32::<LittleEndian>()?;

        // 目录区：每项 8 字节（id + 相对数据区 offset）
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let id = reader.read_u32::<LittleEndian>()?;
            let offset = reader.read_u32::<LittleEndian>()?;
            entries.push((id, offset));
        }

        // 数据区起点 = 文件头 8 字节 + 目录区 count*8 字节
        let data_start = 8 + count as u64 * 8;
        let mut data = vec![0u8; data_size as usize];
        reader.seek(SeekFrom::Start(data_start))?;
        reader.read_exact(&mut data)?;

        let mut strings = HashMap::with_capacity(count as usize);
        for (id, offset) in entries {
            let offset = offset as usize;
            if offset >= data.len() {
                continue;
            }

            let s = match format {
                StringsFormat::NullTerminated => {
                    let start = offset;
                    let mut end = start;
                    while end < data.len() && data[end] != 0 {
                        end += 1;
                    }
                    if end == start {
                        continue;
                    }
                    codepage.decode(&data[start..end])
                }
                StringsFormat::LengthPrefixed => {
                    if offset + 4 > data.len() {
                        continue;
                    }
                    let str_len = u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]) as usize;
                    if str_len == 0 || offset + 4 + str_len.saturating_sub(1) > data.len() {
                        continue;
                    }
                    let content_len = str_len.saturating_sub(1);
                    let start = offset + 4;
                    let end = start + content_len;
                    if end > data.len() {
                        continue;
                    }
                    codepage.decode(&data[start..end])
                }
            };

            if !s.is_empty() {
                strings.insert(id, s);
            }
        }

        Ok(Self {
            strings,
            format,
            codepage,
        })
    }

    pub fn get(&self, id: u32) -> Option<&String> {
        self.strings.get(&id)
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        self.save_with_format(path, self.format)
    }

    pub fn save_with_format<P: AsRef<Path>>(
        &self,
        path: P,
        format: StringsFormat,
    ) -> std::io::Result<()> {
        let path_ref = path.as_ref();
        let mut file = File::create(path_ref)?;
        let mut writer = std::io::BufWriter::new(&mut file);

        // 写出时按 id 升序，保持稳定输出，便于对比和复现。
        let mut entries: Vec<(&u32, &String)> = self.strings.iter().collect();
        entries.sort_by_key(|(id, _)| **id);

        let count = entries.len() as u32;
        let _directory_size = count as usize * 8;

        // 先构造完整数据区，再回写每个条目的 offset（相对数据区起点）。
        // 对相同内容去重：多条目指向同一数据区偏移，缩小文件体积 ~17%。
        let mut data_section = Vec::new();
        let mut dedup: HashMap<Vec<u8>, u32> = HashMap::new();
        let mut offsets: Vec<u32> = Vec::with_capacity(entries.len());

        for (_id, text) in &entries {
            // 使用 codepage 编码字符串
            let bytes = self.codepage.encode(text);
            let entry_data = match format {
                StringsFormat::NullTerminated => {
                    // STRINGS：纯文本 + 0 终止符
                    let mut data = bytes;
                    data.push(0);
                    data
                }
                StringsFormat::LengthPrefixed => {
                    // DLSTRINGS/ILSTRINGS：长度(含结尾0) + 文本 + 结尾0
                    let total_len = (bytes.len() + 1) as u32;
                    let mut data = Vec::with_capacity(4 + bytes.len() + 1);
                    data.extend_from_slice(&total_len.to_le_bytes());
                    data.extend_from_slice(&bytes);
                    data.push(0);
                    data
                }
            };

            if let Some(&existing_offset) = dedup.get(&entry_data) {
                offsets.push(existing_offset);
            } else {
                let offset = data_section.len() as u32;
                offsets.push(offset);
                dedup.insert(entry_data.clone(), offset);
                data_section.extend_from_slice(&entry_data);
            }
        }

        let data_size = data_section.len() as u32;

        // 写文件布局：头(count,data_size) -> 目录(id,offset)*N -> 数据区
        writer.write_u32::<LittleEndian>(count)?;
        writer.write_u32::<LittleEndian>(data_size)?;

        for (i, (id, _text)) in entries.iter().enumerate() {
            writer.write_u32::<LittleEndian>(**id)?;
            writer.write_u32::<LittleEndian>(offsets[i])?;
        }

        writer.write_all(&data_section)?;
        writer.flush()?;

        Ok(())
    }

    pub fn detect_format(path: &Path) -> StringsFormat {
        // 以扩展名判定格式：dlstrings/ilstrings 为长度前缀，其余默认 null 终止。
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
        {
            Some(ref ext) if ext == "dlstrings" || ext == "ilstrings" => {
                StringsFormat::LengthPrefixed
            }
            _ => StringsFormat::NullTerminated,
        }
    }
}

impl Default for StringsFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_null_terminated_data(entries: &[(u32, &str)]) -> Vec<u8> {
        let mut data_section = Vec::new();
        let mut offsets = Vec::new();

        for (_id, text) in entries {
            offsets.push(data_section.len() as u32);
            data_section.extend_from_slice(text.as_bytes());
            data_section.push(0);
        }

        let count = entries.len() as u32;
        let data_size = data_section.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&data_size.to_le_bytes());
        for (i, (id, _text)) in entries.iter().enumerate() {
            buf.extend_from_slice(&id.to_le_bytes());
            buf.extend_from_slice(&offsets[i].to_le_bytes());
        }
        buf.extend_from_slice(&data_section);
        buf
    }

    fn build_length_prefixed_data(entries: &[(u32, &str)]) -> Vec<u8> {
        let mut data_section = Vec::new();
        let mut offsets = Vec::new();

        for (_id, text) in entries {
            offsets.push(data_section.len() as u32);
            let text_bytes = text.as_bytes();
            let total_len = (text_bytes.len() + 1) as u32;
            data_section.extend_from_slice(&total_len.to_le_bytes());
            data_section.extend_from_slice(text_bytes);
            data_section.push(0);
        }

        let count = entries.len() as u32;
        let data_size = data_section.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&data_size.to_le_bytes());
        for (i, (id, _text)) in entries.iter().enumerate() {
            buf.extend_from_slice(&id.to_le_bytes());
            buf.extend_from_slice(&offsets[i].to_le_bytes());
        }
        buf.extend_from_slice(&data_section);
        buf
    }

    #[test]
    fn test_null_terminated_format() {
        let cursor =
            build_null_terminated_data(&[(1u32, "Hello"), (2u32, "World"), (3u32, "Skyrim")]);
        let mut tmp = std::env::temp_dir();
        tmp.push("test_strings_format_check.strings");
        std::fs::write(&tmp, &cursor).unwrap();
        let sf = StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        assert_eq!(sf.strings.len(), 3);
        assert_eq!(sf.strings.get(&1).unwrap(), "Hello");
        assert_eq!(sf.strings.get(&2).unwrap(), "World");
        assert_eq!(sf.strings.get(&3).unwrap(), "Skyrim");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_length_prefixed_format() {
        let cursor =
            build_length_prefixed_data(&[(10u32, "Long text here"), (20u32, "Another entry")]);
        let mut tmp = std::env::temp_dir();
        tmp.push("test_dlstrings_format_check.dlstrings");
        std::fs::write(&tmp, &cursor).unwrap();
        let sf = StringsFile::load_with_format(&tmp, StringsFormat::LengthPrefixed).unwrap();
        assert_eq!(sf.strings.len(), 2);
        assert_eq!(sf.strings.get(&10).unwrap(), "Long text here");
        assert_eq!(sf.strings.get(&20).unwrap(), "Another entry");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_detect_format_from_extension() {
        assert_eq!(
            StringsFile::detect_format(Path::new("skyrim_english.strings")),
            StringsFormat::NullTerminated
        );
        assert_eq!(
            StringsFile::detect_format(Path::new("skyrim_english.dlstrings")),
            StringsFormat::LengthPrefixed
        );
        assert_eq!(
            StringsFile::detect_format(Path::new("skyrim_english.ILSTRINGS")),
            StringsFormat::LengthPrefixed
        );
    }

    #[test]
    fn test_chinese_strings() {
        let cursor = build_null_terminated_data(&[
            (75388u32, "魔人"),
            (75234u32, "盗贼"),
            (75148u32, "魔剑士"),
        ]);
        let mut tmp = std::env::temp_dir();
        tmp.push("test_chinese_strings.strings");
        std::fs::write(&tmp, &cursor).unwrap();
        let sf = StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        assert_eq!(sf.strings.len(), 3);
        assert_eq!(sf.strings.get(&75388).unwrap(), "魔人");
        assert_eq!(sf.strings.get(&75234).unwrap(), "盗贼");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_dlstrings_roundtrip() {
        let entries = vec![
            (100u32, "Hello World"),
            (200u32, "DLSTRINGS test"),
            (300u32, "中文测试"),
        ];
        let data = build_length_prefixed_data(&entries);
        let mut tmp = std::env::temp_dir();
        tmp.push("test_dlstrings_rt.dlstrings");
        std::fs::write(&tmp, &data).unwrap();
        let sf = StringsFile::load_with_format(&tmp, StringsFormat::LengthPrefixed).unwrap();
        assert_eq!(sf.strings.len(), 3);
        assert_eq!(sf.strings.get(&100).unwrap(), "Hello World");
        assert_eq!(sf.strings.get(&200).unwrap(), "DLSTRINGS test");
        assert_eq!(sf.strings.get(&300).unwrap(), "中文测试");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_save_null_terminated_roundtrip() {
        let mut sf = StringsFile::new();
        sf.strings.insert(1, "Hello".to_string());
        sf.strings.insert(5, "World".to_string());
        sf.strings.insert(10, "Skyrim".to_string());
        sf.format = StringsFormat::NullTerminated;

        let mut tmp = std::env::temp_dir();
        tmp.push("test_save_null_rt.strings");
        sf.save(&tmp).unwrap();

        let loaded = StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        assert_eq!(loaded.strings.len(), 3);
        assert_eq!(loaded.strings.get(&1).unwrap(), "Hello");
        assert_eq!(loaded.strings.get(&5).unwrap(), "World");
        assert_eq!(loaded.strings.get(&10).unwrap(), "Skyrim");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_save_length_prefixed_roundtrip() {
        let mut sf = StringsFile::new();
        sf.strings.insert(100, "Hello World".to_string());
        sf.strings.insert(200, "DLSTRINGS test".to_string());
        sf.strings.insert(300, "中文测试".to_string());
        sf.format = StringsFormat::LengthPrefixed;

        let mut tmp = std::env::temp_dir();
        tmp.push("test_save_lp_rt.dlstrings");
        sf.save(&tmp).unwrap();

        let loaded = StringsFile::load_with_format(&tmp, StringsFormat::LengthPrefixed).unwrap();
        assert_eq!(loaded.strings.len(), 3);
        assert_eq!(loaded.strings.get(&100).unwrap(), "Hello World");
        assert_eq!(loaded.strings.get(&200).unwrap(), "DLSTRINGS test");
        assert_eq!(loaded.strings.get(&300).unwrap(), "中文测试");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_save_overwrite_roundtrip() {
        let mut sf = StringsFile::new();
        sf.strings.insert(1, "Original".to_string());
        sf.strings.insert(2, "Second".to_string());
        sf.format = StringsFormat::NullTerminated;

        let mut tmp = std::env::temp_dir();
        tmp.push("test_save_overwrite.strings");
        sf.save(&tmp).unwrap();

        let mut loaded =
            StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        loaded.strings.insert(1, "Modified".to_string());
        loaded.strings.insert(3, "New entry".to_string());
        loaded.save(&tmp).unwrap();

        let reloaded = StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        assert_eq!(reloaded.strings.len(), 3);
        assert_eq!(reloaded.strings.get(&1).unwrap(), "Modified");
        assert_eq!(reloaded.strings.get(&2).unwrap(), "Second");
        assert_eq!(reloaded.strings.get(&3).unwrap(), "New entry");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_save_empty_strings_file() {
        let sf = StringsFile::new();

        let mut tmp = std::env::temp_dir();
        tmp.push("test_save_empty.strings");
        sf.save(&tmp).unwrap();

        let loaded = StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        assert!(loaded.strings.is_empty());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_save_deduplication() {
        // 三条条目，其中两条内容相同 → 应共享数据区偏移
        let mut sf = StringsFile::new();
        sf.strings.insert(1, "Duplicate".to_string());
        sf.strings.insert(2, "Unique".to_string());
        sf.strings.insert(3, "Duplicate".to_string()); // same as id=1
        sf.format = StringsFormat::NullTerminated;

        let mut tmp = std::env::temp_dir();
        tmp.push("test_save_dedup.strings");
        sf.save(&tmp).unwrap();

        let loaded = StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        assert_eq!(loaded.strings.len(), 3);
        assert_eq!(loaded.strings.get(&1).unwrap(), "Duplicate");
        assert_eq!(loaded.strings.get(&2).unwrap(), "Unique");
        assert_eq!(loaded.strings.get(&3).unwrap(), "Duplicate");

        // 去重后的文件应比不区分的更小
        let size = std::fs::metadata(&tmp).unwrap().len();
        // 3 entries: header(8) + directory(3*8=24) + data(Duplicate\0=10 + Unique\0=7) = 49
        // Without dedup it would be: 8 + 24 + 10 + 7 + 10 = 59
        assert!(size < 55, "Dedup should produce smaller file, got {size} bytes");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_load_with_codepage_fallback() {
        // 模拟 CP1252 编码的数据：0x80 = €
        let mut data_section = Vec::new();
        let offset0 = 0u32;
        data_section.push(0x80); // € in CP1252, not valid UTF-8
        data_section.push(0); // null terminator

        let count = 1u32;
        let data_size = data_section.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&data_size.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // id
        buf.extend_from_slice(&offset0.to_le_bytes()); // offset
        buf.extend_from_slice(&data_section);

        let mut tmp = std::env::temp_dir();
        tmp.push("test_cp1252_fallback.strings");
        std::fs::write(&tmp, &buf).unwrap();

        // 无 fallback → 逐字节解码
        let sf = StringsFile::load_with_format(&tmp, StringsFormat::NullTerminated).unwrap();
        assert_eq!(sf.strings.get(&1).unwrap(), "\u{0080}");

        // 有 CP1252 fallback → 正确解码
        let config = CodepageConfig::utf8_with_fallback(CodepageId::Cp1252);
        let sf2 = StringsFile::load_with_codepage(&tmp, config).unwrap();
        assert_eq!(sf2.strings.get(&1).unwrap(), "€");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_load_with_codepage_table() {
        let content = "english=utf8,1252\njapanese=utf8,932\n";
        let table = CodepageTable::parse(content);

        // UTF-8 文件（正常）
        let cursor = build_null_terminated_data(&[(1u32, "Hello")]);
        let mut tmp = std::env::temp_dir();
        tmp.push("test_cp_english.strings");
        std::fs::write(&tmp, &cursor).unwrap();

        let sf = StringsFile::load_with_codepage_table(&tmp, &table).unwrap();
        assert_eq!(sf.strings.get(&1).unwrap(), "Hello");

        let _ = std::fs::remove_file(&tmp);
    }
}
