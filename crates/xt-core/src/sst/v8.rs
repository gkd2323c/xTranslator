use crate::sst::encoding::{read_delphi_string, write_delphi_string};
use crate::types::esp_pointer::EspPointer;
use crate::types::params::SkyStringParams;
use crate::types::sky_string::SkyString;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Result, Write};
use std::path::Path;

/// SST v8 魔数: $39555353 (little-endian bytes: 53 55 53 39)
pub const SST_V8_MAGIC: u32 = 0x39555353;

/// SST 字典结构
#[derive(Clone, Debug)]
pub struct SstDictionary {
    /// Master 文件列表 (v8+)
    pub master_list: Vec<String>,
    /// Colab 标签列表: (id, label)
    pub colab_labels: Vec<(u32, String)>,
    /// 字符串条目
    pub entries: Vec<SkyString>,
}

impl SstDictionary {
    pub fn new() -> Self {
        Self {
            master_list: Vec::new(),
            colab_labels: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// 从 SkyString 列表创建 SST 字典
    pub fn from_entries(entries: Vec<SkyString>) -> Self {
        Self {
            master_list: Vec::new(),
            colab_labels: Vec::new(),
            entries,
        }
    }

    /// 从 SkyString 列表创建 SST 字典，指定 master 文件
    pub fn from_entries_with_masters(entries: Vec<SkyString>, masters: Vec<String>) -> Self {
        Self {
            master_list: masters,
            colab_labels: Vec::new(),
            entries,
        }
    }

    /// 从文件读取 SST
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let mut reader = BufReader::new(file);
        Self::read_from(&mut reader)
    }

    /// 保存 SST 到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);
        self.write_to(&mut writer)
    }

    /// 读取 SST v8 文件
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        // 1. 魔数
        let magic = reader.read_u32::<LittleEndian>()?;
        if magic != SST_V8_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Invalid SST magic: expected {:08X}, got {:08X}",
                    SST_V8_MAGIC, magic
                ),
            ));
        }

        let mut dict = Self::new();

        // 2. v4 占位符 (1 byte)
        let _v4_flag = reader.read_u8()?;

        // 3. Master List (v8+)
        let master_count = reader.read_i32::<LittleEndian>()?;
        for _ in 0..master_count {
            let s = read_delphi_string(reader)?;
            dict.master_list.push(s);
        }

        // 4. Colab Label List (v7+)
        let colab_count = reader.read_i32::<LittleEndian>()?;
        for _ in 0..colab_count {
            let id = reader.read_i32::<LittleEndian>()? as u32;
            let label = read_delphi_string(reader)?;
            dict.colab_labels.push((id, label));
        }

        // 5. 字符串条目 (循环到 EOF)
        loop {
            // 尝试读取 listIndex (1 byte)
            let mut list_index_buf = [0u8; 1];
            match reader.read_exact(&mut list_index_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
            let list_index = list_index_buf[0];

            // 读取 EspPointerLite (24 bytes)
            let str_id = reader.read_i32::<LittleEndian>()?;
            let form_id = reader.read_u32::<LittleEndian>()?;
            let mut record_sig = [0u8; 4];
            reader.read_exact(&mut record_sig)?;
            let mut field_sig = [0u8; 4];
            reader.read_exact(&mut field_sig)?;
            let index = reader.read_u16::<LittleEndian>()?;
            let index_max = reader.read_u16::<LittleEndian>()?;
            let edid_hash = reader.read_u32::<LittleEndian>()?;

            let esp_ptr = EspPointer {
                str_id,
                form_id,
                record_sig,
                field_sig,
                index,
                index_max,
                edid_hash,
            };

            // colabId (v6+)
            let colab_id = reader.read_u8()?;

            // sparams (1 byte)
            let params_byte = reader.read_u8()?;
            let params = SkyStringParams(params_byte);

            // source string
            let source = read_delphi_string(reader)?;

            // translation string
            let translation = read_delphi_string(reader)?;

            let mut sk = SkyString::new(0, source, translation, *b"UNKN", *b"UNKN");
            sk.esp_ptr = esp_ptr;
            sk.params = params;
            sk.colab_id = colab_id;
            sk.list_index = list_index;

            dict.entries.push(sk);
        }

        Ok(dict)
    }

    /// 写入 SST v8 文件
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // 1. 魔数
        writer.write_u32::<LittleEndian>(SST_V8_MAGIC)?;

        // 2. v4 占位符
        writer.write_u8(0)?;

        // 3. Master List
        writer.write_i32::<LittleEndian>(self.master_list.len() as i32)?;
        for master in &self.master_list {
            write_delphi_string(writer, master)?;
        }

        // 4. Colab Label List
        writer.write_i32::<LittleEndian>(self.colab_labels.len() as i32)?;
        for (id, label) in &self.colab_labels {
            writer.write_i32::<LittleEndian>(*id as i32)?;
            write_delphi_string(writer, label)?;
        }

        // 5. 字符串条目
        for sk in &self.entries {
            // listIndex
            writer.write_u8(sk.list_index)?;

            // EspPointerLite (24 bytes)
            sk.esp_ptr.write_to(writer)?;

            // colabId
            writer.write_u8(sk.colab_id)?;

            // sparams (移除 validated 标志，如 Delphi 所做)
            let params = sk.params.0 & !SkyStringParams::VALIDATED;
            writer.write_u8(params)?;

            // source
            write_delphi_string(writer, &sk.source)?;

            // translation
            write_delphi_string(writer, &sk.translation)?;
        }

        Ok(())
    }
}

impl Default for SstDictionary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sst_roundtrip() {
        let mut dict = SstDictionary::new();
        dict.master_list = vec!["Skyrim.esm".to_string(), "Update.esm".to_string()];
        dict.colab_labels = vec![
            (1, "TranslatorA".to_string()),
            (2, "TranslatorB".to_string()),
        ];

        for i in 0..10 {
            let mut sk = SkyString::new(
                i,
                format!("Source {}", i),
                format!("Translation {}", i),
                *b"INFO",
                *b"DESC",
            );
            sk.esp_ptr.record_sig = *b"INFO";
            sk.esp_ptr.field_sig = *b"NAM1";
            sk.esp_ptr.form_id = 0x01000000 + i as u32;
            sk.esp_ptr.str_id = i as i32;
            sk.colab_id = (i % 3) as u8;
            sk.list_index = (i % 3) as u8;
            sk.params.set(SkyStringParams::TRANSLATED, i % 2 == 0);
            dict.entries.push(sk);
        }

        // Write
        let mut buf = Vec::new();
        dict.write_to(&mut buf).unwrap();

        // Read back
        let dict2 = SstDictionary::read_from(&mut buf.as_slice()).unwrap();

        // Verify
        assert_eq!(dict.master_list, dict2.master_list);
        assert_eq!(dict.colab_labels.len(), dict2.colab_labels.len());
        assert_eq!(dict.entries.len(), dict2.entries.len());

        for (a, b) in dict.entries.iter().zip(dict2.entries.iter()) {
            assert_eq!(a.source, b.source);
            assert_eq!(a.translation, b.translation);
            assert_eq!(a.esp_ptr, b.esp_ptr);
            assert_eq!(a.colab_id, b.colab_id);
            assert_eq!(a.list_index, b.list_index);
        }
    }

    #[test]
    fn test_sst_unicode_roundtrip() {
        let mut dict = SstDictionary::new();
        let mut sk = SkyString::new(
            0,
            "你好世界".to_string(),
            "Hello World".to_string(),
            *b"BOOK",
            *b"DESC",
        );
        sk.esp_ptr.record_sig = *b"BOOK";
        sk.esp_ptr.field_sig = *b"DESC";
        dict.entries.push(sk);

        let mut buf = Vec::new();
        dict.write_to(&mut buf).unwrap();

        let dict2 = SstDictionary::read_from(&mut buf.as_slice()).unwrap();
        assert_eq!(dict2.entries[0].source, "你好世界");
        assert_eq!(dict2.entries[0].translation, "Hello World");
    }

    #[test]
    fn test_sst_empty() {
        let dict = SstDictionary::new();
        let mut buf = Vec::new();
        dict.write_to(&mut buf).unwrap();

        let dict2 = SstDictionary::read_from(&mut buf.as_slice()).unwrap();
        assert!(dict2.entries.is_empty());
        assert!(dict2.master_list.is_empty());
        assert!(dict2.colab_labels.is_empty());
    }
}
