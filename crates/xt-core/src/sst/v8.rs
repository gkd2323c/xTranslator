use crate::sst::encoding::{read_delphi_string, write_delphi_string};
use crate::types::esp_pointer::EspPointer;
use crate::types::params::SkyStringParams;
use crate::types::sky_string::SkyString;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Result, Write};
use std::path::Path;

/// SST 版本魔数定义 (Little-Endian)
/// 用于标识不同版本的 SST 文件格式
mod sst_magic {
    /// v1: SSU2 (基础格式)
    pub const V1: u32 = 0x32555353; // $32555353
    /// v2: SSU3
    pub const V2: u32 = 0x33555353; // $33555353
    /// v3: SSU4
    pub const V3: u32 = 0x34555353; // $34555353
    /// v4: SSU5
    pub const V4: u32 = 0x35555353; // $35555353
    /// v5: SSU6 (添加 real edidHash)
    pub const V5: u32 = 0x36555353; // $36555353
    /// v6: SSU7 (添加 colabId)
    pub const V6: u32 = 0x37555353; // $37555353
    /// v7: SSU8 (添加 colab label)
    pub const V7: u32 = 0x38555353; // $38555353
    /// v8: SSU9 (添加 master list)
    pub const V8: u32 = 0x39555353; // $39555353
    /// 当前最新版本
    pub const CURRENT: u32 = V8;
}

/// SST 版本号
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SstVersion {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
}

impl SstVersion {
    /// 从魔数获取版本号
    pub fn from_magic(magic: u32) -> Option<Self> {
        match magic {
            0x32555353 => Some(Self::V1),
            0x33555353 => Some(Self::V2),
            0x34555353 => Some(Self::V3),
            0x35555353 => Some(Self::V4),
            0x36555353 => Some(Self::V5),
            0x37555353 => Some(Self::V6),
            0x38555353 => Some(Self::V7),
            0x39555353 => Some(Self::V8),
            _ => None,
        }
    }

    /// 获取版本号 (1-8)
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
            Self::V4 => 4,
            Self::V5 => 5,
            Self::V6 => 6,
            Self::V7 => 7,
            Self::V8 => 8,
        }
    }

    /// 获取版本比较值（用于 >= 比较）
    fn cmp_value(&self) -> u32 {
        self.as_u32()
    }
}

impl std::fmt::Display for SstVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.as_u32())
    }
}

/// SST v8 魔数: $39555353 (little-endian bytes: 53 55 53 39)
/// 对应 ASCII: "9USS"（倒序）
pub const SST_V8_MAGIC: u32 = sst_magic::V8;

/// SST 字典结构
///
/// SST（Sky String Table）是 xTranslator 的核心字典格式，用于：
/// - 保存翻译工作（跨会话持久化）
/// - 导入/导出翻译数据
/// - 与其他工具交互
///
/// 格式演进：
/// - v1-v5: 基础格式
/// - v6: 添加 colabId（协作 ID）
/// - v7: 添加 Colab Label List
/// - v8: 添加 Master List（游戏主文件列表）
///
/// 当前使用 v8 格式，向后兼容 v1-v7。
#[derive(Clone, Debug)]
pub struct SstDictionary {
    /// 读取的 SST 版本
    pub version: Option<SstVersion>,
    /// Master 文件列表 (v8+)
    /// 记录 SST 创建时的游戏主文件（如 "Skyrim.esm", "Update.esm"）
    /// 用于验证 SST 与当前游戏版本的兼容性
    pub master_list: Vec<String>,
    /// Colab 标签列表: (id, label)
    /// 用于多人协作翻译时的身份标识
    /// 例如：(1, "Alice"), (2, "Bob")
    pub colab_labels: Vec<(u32, String)>,
    /// 字符串条目
    /// 每个条目对应一个可翻译字符串及其翻译
    pub entries: Vec<SkyString>,
}

impl SstDictionary {
    pub fn new() -> Self {
        Self {
            version: None,
            master_list: Vec::new(),
            colab_labels: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// 从 SkyString 列表创建 SST 字典
    pub fn from_entries(entries: Vec<SkyString>) -> Self {
        Self {
            version: Some(SstVersion::V8),
            master_list: Vec::new(),
            colab_labels: Vec::new(),
            entries,
        }
    }

    /// 从 SkyString 列表创建 SST 字典，指定 master 文件
    ///
    /// 参数：
    /// - `entries`: 字符串条目列表
    /// - `masters`: 游戏主文件列表（如 ["Skyrim.esm", "Update.esm"]）
    pub fn from_entries_with_masters(entries: Vec<SkyString>, masters: Vec<String>) -> Self {
        Self {
            version: Some(SstVersion::V8),
            master_list: masters,
            colab_labels: Vec::new(),
            entries,
        }
    }

    /// 从文件读取 SST
    ///
    /// 自动检测版本并解析。支持 v6-v8 格式。
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let mut reader = BufReader::new(file);
        Self::read_from(&mut reader)
    }

    /// 保存 SST 到文件
    ///
    /// 总是使用 v8 格式保存，确保最大兼容性。
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);
        self.write_to(&mut writer)
    }

    /// 读取 SST 文件（支持 v1-v8 所有版本）
    ///
    /// 根据魔数自动检测版本，然后按各版本规范解析。
    ///
    /// 各版本差异：
    /// - v1-v2: 只有 listIndex + sparams + source + translation
    /// - v3+: 添加 str_id, form_id
    /// - v4+: 添加 index (v3 有可选), indexMax + edidHash
    /// - v5+: 添加 colabId
    /// - v6+: 添加 colabId 到条目
    /// - v7+: 添加 Colab Label List (文件级别)
    /// - v8+: 添加 Master List (文件级别)
    ///
    /// v1-v5 的 EspPointer 字段布局：
    /// - v1: 无 str_id/form_id，直接从 sparams 开始
    /// - v2: 无 str_id/form_id，直接从 sparams 开始
    /// - v3: str_id(4) + form_id(4) + field_sig(4) [+ index(2) 可选] + sparams
    /// - v4: str_id(4) + form_id(4) + record_sig(4) + field_sig(4) + index(2) + indexMax(2) + edidHash(4) + sparams
    /// - v5: 同 v4 + colabId
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        // 1. 魔数 - 用于确定版本
        let magic = reader.read_u32::<LittleEndian>()?;
        let version = SstVersion::from_magic(magic).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown SST magic: {:08X}", magic),
            )
        })?;

        let mut dict = Self::new();
        dict.version = Some(version);

        // 2. v4 占位符 (v3+ 有这个字节)
        if version.as_u32() >= 3 {
            let _v4_flag = reader.read_u8()?;
        }

        // 3. Master List (v8+)
        if version >= SstVersion::V8 {
            let master_count = reader.read_i32::<LittleEndian>()?;
            for _ in 0..master_count {
                let s = read_delphi_string(reader)?;
                dict.master_list.push(s);
            }
        }

        // 4. Colab Label List (v7+)
        if version >= SstVersion::V7 {
            let colab_count = reader.read_i32::<LittleEndian>()?;
            for _ in 0..colab_count {
                let id = reader.read_i32::<LittleEndian>()? as u32;
                let label = read_delphi_string(reader)?;
                dict.colab_labels.push((id, label));
            }
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

            // 读取 EspPointer 并填充各字段
            let mut esp_ptr = EspPointer::null();
            let mut colab_id = 0u8;

            // v2+: 有 str_id 和 form_id
            if version >= SstVersion::V2 {
                esp_ptr.str_id = reader.read_i32::<LittleEndian>()?;
                esp_ptr.form_id = reader.read_u32::<LittleEndian>()?;
            }

            // v4+: 有 record_sig
            if version >= SstVersion::V4 {
                reader.read_exact(&mut esp_ptr.record_sig)?;
            }

            // 所有版本都有 field_sig
            reader.read_exact(&mut esp_ptr.field_sig)?;

            // v3+: 有 index (但 v3 是可选的，这里简化处理)
            if version >= SstVersion::V3 {
                esp_ptr.index = reader.read_u16::<LittleEndian>()?;
            }

            // v4+: 有 index_max 和 edid_hash
            if version >= SstVersion::V4 {
                esp_ptr.index_max = reader.read_u16::<LittleEndian>()?;
                esp_ptr.edid_hash = reader.read_u32::<LittleEndian>()?;
            }

            // v6+: 条目中有 colabId
            if version >= SstVersion::V6 {
                colab_id = reader.read_u8()?;
            }

            // sparams (1 byte) - 所有版本都有
            let params_byte = reader.read_u8()?;
            let params = SkyStringParams(params_byte);

            // source string
            let source = read_delphi_string(reader)?;

            // translation string
            let translation = read_delphi_string(reader)?;

            // 跳过空翻译（与 Delphi 一致）
            if source.is_empty() && translation.is_empty() {
                continue;
            }

            let mut sk = SkyString::new(
                0,
                source,
                translation,
                esp_ptr.record_sig,
                esp_ptr.field_sig,
            );
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

    /// 将另一个 SST 字典的翻译合并到当前字典
    ///
    /// 匹配策略：按 `(str_id, record_sig, field_sig)` 三元组匹配条目。
    ///
    /// 冲突处理（当两边都有非空译文时）：
    /// - `overwrite=false`：保留当前译文
    /// - `overwrite=true`：用来源译文覆盖
    ///
    /// 返回合并统计。
    pub fn merge_from(&mut self, other: &SstDictionary, overwrite: bool) -> MergeStats {
        let mut stats = MergeStats::default();

        // 构建当前条目的索引：(str_id, record_sig, field_sig) -> entry_index
        let mut index: std::collections::HashMap<(i32, [u8; 4], [u8; 4]), usize> =
            std::collections::HashMap::with_capacity(self.entries.len());
        for (i, entry) in self.entries.iter().enumerate() {
            index.insert(
                (
                    entry.esp_ptr.str_id,
                    entry.esp_ptr.record_sig,
                    entry.esp_ptr.field_sig,
                ),
                i,
            );
        }

        for other_entry in &other.entries {
            let key = (
                other_entry.esp_ptr.str_id,
                other_entry.esp_ptr.record_sig,
                other_entry.esp_ptr.field_sig,
            );

            if let Some(&idx) = index.get(&key) {
                // 条目在两方都存在
                let target = &mut self.entries[idx];
                let source_has_trans = !other_entry.translation.is_empty();
                let target_has_trans = !target.translation.is_empty();

                if source_has_trans && !target_has_trans {
                    // 来源有译文，当前无 → 复制
                    target.translation = other_entry.translation.clone();
                    target.params = other_entry.params;
                    stats.updated += 1;
                } else if source_has_trans && target_has_trans {
                    if overwrite && target.translation != other_entry.translation {
                        target.translation = other_entry.translation.clone();
                        target.params = other_entry.params;
                        stats.overwritten += 1;
                    } else {
                        stats.conflicts_skipped += 1;
                    }
                }
                // 来源没有译文：跳过
            } else {
                // 条目仅在来源中存在 → 添加
                self.entries.push(other_entry.clone());
                stats.added += 1;
            }
        }

        // 合并 master list（去重）
        for master in &other.master_list {
            if !self.master_list.contains(master) {
                self.master_list.push(master.clone());
            }
        }

        // 合并 colab labels（去重，按 id）
        for (id, label) in &other.colab_labels {
            if !self.colab_labels.iter().any(|(cid, _)| cid == id) {
                self.colab_labels.push((*id, label.clone()));
            }
        }

        stats
    }
}

/// SST 合并统计
#[derive(Debug, Clone, Default)]
pub struct MergeStats {
    /// 新增条目数（仅在来源中存在）
    pub added: usize,
    /// 更新条目数（当前无译文，来源有译文）
    pub updated: usize,
    /// 覆盖条目数（双方都有译文，被来源覆盖）
    pub overwritten: usize,
    /// 跳过冲突数（双方都有译文但未覆盖）
    pub conflicts_skipped: usize,
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

        // 写入
        let mut buf = Vec::new();
        dict.write_to(&mut buf).unwrap();

        // 读回
        let dict2 = SstDictionary::read_from(&mut buf.as_slice()).unwrap();

        // 验证
        assert_eq!(dict2.version, Some(SstVersion::V8));
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

    #[test]
    fn test_merge_new_entries() {
        let mut target = SstDictionary::new();
        target.entries.push(SkyString::new(
            0,
            "hello".into(),
            "".into(),
            *b"INFO",
            *b"FULL",
        ));
        target.entries[0].esp_ptr.str_id = 1;

        let mut source = SstDictionary::new();
        source.entries.push(SkyString::new(
            0,
            "world".into(),
            "世界".into(),
            *b"INFO",
            *b"FULL",
        ));
        source.entries[0].esp_ptr.str_id = 2;

        let stats = target.merge_from(&source, false);
        assert_eq!(stats.added, 1);
        assert_eq!(target.entries.len(), 2);
    }

    #[test]
    fn test_merge_update_existing() {
        let mut target = SstDictionary::new();
        target.entries.push(SkyString::new(
            0,
            "hello".into(),
            "".into(),
            *b"INFO",
            *b"FULL",
        ));
        target.entries[0].esp_ptr.str_id = 1;

        let mut source = SstDictionary::new();
        source.entries.push(SkyString::new(
            0,
            "hello".into(),
            "你好".into(),
            *b"INFO",
            *b"FULL",
        ));
        source.entries[0].esp_ptr.str_id = 1;

        let stats = target.merge_from(&source, false);
        assert_eq!(stats.updated, 1);
        assert_eq!(target.entries[0].translation, "你好");
    }

    #[test]
    fn test_merge_conflict_no_overwrite() {
        let mut target = SstDictionary::new();
        target.entries.push(SkyString::new(
            0,
            "hello".into(),
            "你好".into(),
            *b"INFO",
            *b"FULL",
        ));
        target.entries[0].esp_ptr.str_id = 1;

        let mut source = SstDictionary::new();
        source.entries.push(SkyString::new(
            0,
            "hello".into(),
            "您好".into(),
            *b"INFO",
            *b"FULL",
        ));
        source.entries[0].esp_ptr.str_id = 1;

        let stats = target.merge_from(&source, false);
        assert_eq!(stats.conflicts_skipped, 1);
        assert_eq!(target.entries[0].translation, "你好");
    }

    #[test]
    fn test_merge_conflict_with_overwrite() {
        let mut target = SstDictionary::new();
        target.entries.push(SkyString::new(
            0,
            "hello".into(),
            "你好".into(),
            *b"INFO",
            *b"FULL",
        ));
        target.entries[0].esp_ptr.str_id = 1;

        let mut source = SstDictionary::new();
        source.entries.push(SkyString::new(
            0,
            "hello".into(),
            "您好".into(),
            *b"INFO",
            *b"FULL",
        ));
        source.entries[0].esp_ptr.str_id = 1;

        let stats = target.merge_from(&source, true);
        assert_eq!(stats.overwritten, 1);
        assert_eq!(target.entries[0].translation, "您好");
    }

    #[test]
    fn test_read_sst_v4_format() {
        use crate::sst::encoding::write_delphi_string;

        // v4 格式结构（基于 Delphi loadVocabUserCache）：
        // - list_index(1)
        // - str_id(4), form_id(4) [v2+]
        // - record_sig(4), field_sig(4) [v4+]
        // - index(2), index_max(2), edid_hash(4) [v4+]
        // - sparams(1)
        // - source, translation [Delphi 字符串]
        let mut buf = Vec::new();
        buf.extend_from_slice(&0x35555353u32.to_le_bytes()); // v4 magic
        buf.push(0); // v4 placeholder flag
        buf.push(1); // list_index = .DLSTRINGS
        buf.extend_from_slice(&1i32.to_le_bytes()); // str_id
        buf.extend_from_slice(&0x01000001u32.to_le_bytes()); // form_id
        buf.extend_from_slice(b"QUST"); // record_sig
        buf.extend_from_slice(b"FULL"); // field_sig
        buf.extend_from_slice(&1u16.to_le_bytes()); // index
        buf.extend_from_slice(&10u16.to_le_bytes()); // index_max
        buf.extend_from_slice(&0x12345678u32.to_le_bytes()); // edid_hash
        buf.push(0x01); // sparams
        write_delphi_string(&mut buf, "Quest Name").unwrap();
        write_delphi_string(&mut buf, "任务名称").unwrap();

        let dict = SstDictionary::read_from(&mut buf.as_slice()).unwrap();
        assert_eq!(dict.version, Some(SstVersion::V4));
        assert_eq!(dict.entries.len(), 1);
        assert_eq!(dict.entries[0].source, "Quest Name");
        assert_eq!(dict.entries[0].esp_ptr.str_id, 1);
        assert_eq!(dict.entries[0].esp_ptr.record_sig, *b"QUST");
        assert_eq!(dict.entries[0].esp_ptr.edid_hash, 0x12345678);
        assert_eq!(dict.entries[0].list_index, 1);
    }

    #[test]
    fn test_read_sst_v7_format() {
        use crate::sst::encoding::write_delphi_string;

        // v7: 添加 Colab Label List（文件级别）
        let mut buf = Vec::new();
        buf.extend_from_slice(&0x38555353u32.to_le_bytes()); // v7 magic
        buf.push(0); // v4 placeholder flag
                     // Colab Label List (v7+): count(4) + [(id:4 + label)]
        buf.extend_from_slice(&2i32.to_le_bytes()); // 2 labels
        buf.extend_from_slice(&1i32.to_le_bytes()); // label 1 id
        write_delphi_string(&mut buf, "Alice").unwrap();
        buf.extend_from_slice(&2i32.to_le_bytes()); // label 2 id
        write_delphi_string(&mut buf, "Bob").unwrap();
        // 条目
        buf.push(0); // list_index
        buf.extend_from_slice(&5i32.to_le_bytes()); // str_id
        buf.extend_from_slice(&0x02000005u32.to_le_bytes()); // form_id
        buf.extend_from_slice(b"INFO");
        buf.extend_from_slice(b"FULL");
        buf.extend_from_slice(&0u16.to_le_bytes()); // index
        buf.extend_from_slice(&0u16.to_le_bytes()); // index_max
        buf.extend_from_slice(&0u32.to_le_bytes()); // edid_hash
        buf.push(1); // colabId (v6+)
        buf.push(0x01); // sparams
        write_delphi_string(&mut buf, "Hello").unwrap();
        write_delphi_string(&mut buf, "你好").unwrap();

        let dict = SstDictionary::read_from(&mut buf.as_slice()).unwrap();
        assert_eq!(dict.version, Some(SstVersion::V7));
        assert_eq!(dict.colab_labels.len(), 2);
        assert_eq!(dict.colab_labels[0], (1, "Alice".to_string()));
        assert_eq!(dict.entries.len(), 1);
        assert_eq!(dict.entries[0].colab_id, 1);
    }

    #[test]
    fn test_sst_version_enum() {
        assert_eq!(SstVersion::V1.as_u32(), 1);
        assert_eq!(SstVersion::V8.as_u32(), 8);
        assert!(SstVersion::V5 >= SstVersion::V4);
        assert!(SstVersion::V1 < SstVersion::V8);

        assert_eq!(SstVersion::from_magic(0x39555353), Some(SstVersion::V8));
        assert_eq!(SstVersion::from_magic(0x32555353), Some(SstVersion::V1));
        assert_eq!(SstVersion::from_magic(0x99999999), None);
    }
}
