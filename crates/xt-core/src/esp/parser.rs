use crate::esp::header::{FieldHeader, GenericHeader, GrupHeader, RecordHeaderData};
use crate::esp::record_tree::{EspField, EspFile, EspGrup, EspRecord, Tes4Header};
use crate::strings::{CodepageTable, StringsFile, StringsFormat};
use crate::types::esp_pointer::{string_hash, EspPointer};
use crate::types::game_id::GameId;
use crate::types::params::SkyStringParams;
use crate::types::sky_string::SkyString;
use crate::vmad::VmadDecoder;
use std::collections::HashMap;
use std::io::{Cursor, Read, Result};
use std::path::Path;

/// 解压 Bethesda 压缩记录
///
/// Bethesda 特有的压缩格式(用于 Oblivion/Skyrim 等游戏)：
/// - 前4字节：小端序的解压后大小(u32)
/// - 剩余数据：zlib 压缩数据
///
/// 参考 Delphi 实现：DecompressToUserBuf(@b[4], header.dsize - sizeOf(cardinal), ...)
///
/// # 参数
/// * `data` - 压缩记录数据(包含4字节大小头)
///
/// # 返回
/// 解压后的数据，或错误
pub(crate) fn decompress_bethesda_record(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;

    // 至少需要4字节的大小头
    if data.len() < 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Compressed record data too short",
        ));
    }

    // 读取解压后大小(小端序)
    let decompressed_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

    // 大小为0表示空记录
    if decompressed_size == 0 {
        return Ok(Vec::new());
    }

    // 合理性检查：防止异常声明大小导致内存膨胀(常见于损坏文件)。
    if decompressed_size > 100_000_000 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unreasonable decompressed size: {}", decompressed_size),
        ));
    }

    // 压缩体从第 5 字节开始(前 4 字节是解压后大小)。
    let compressed = &data[4..];
    if compressed.is_empty() {
        // 只有大小头，没有压缩数据，返回全0
        return Ok(vec![0u8; decompressed_size]);
    }

    // Bethesda 记录压缩体使用 zlib。
    let mut decoder = ZlibDecoder::new(compressed);
    let mut decompressed = Vec::with_capacity(decompressed_size);
    match decoder.read_to_end(&mut decompressed) {
        Ok(_) => {
            // 解压长度和头部声明可能不一致：记录告警但不直接失败，尽量继续解析。
            if decompressed.len() != decompressed_size {
                eprintln!(
                    "Warning: decompressed size mismatch: expected {}, got {}",
                    decompressed_size,
                    decompressed.len()
                );
            }
            Ok(decompressed)
        }
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Zlib decompression failed: {}", e),
        )),
    }
}

/// 可翻译字段定义
///
/// 从 _recorddefs.txt 解析而来，描述哪些字段包含可翻译的字符串
#[derive(Clone, Debug)]
pub struct TranslatableField {
    /// 记录类型签名(4字节 ASCII，如 "INFO", "QUST")
    pub record_sig: [u8; 4],
    /// 字段签名(4字节 ASCII，如 "NAM1", "FULL")
    pub field_sig: [u8; 4],
    /// Strings 文件类型索引：0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
    pub list_index: u8,
    /// Not-null 标记(*)：字符串不能为空
    pub not_null: bool,
    /// Ignored 标记(?)：此定义应被忽略
    pub ignored: bool,
}

impl TranslatableField {
    pub fn new(record_sig: [u8; 4], field_sig: [u8; 4], list_index: u8) -> Self {
        Self {
            record_sig,
            field_sig,
            list_index,
            not_null: false,
            ignored: false,
        }
    }
}

/// Strings 文件集合
#[derive(Default)]
pub struct StringsFiles {
    pub strings: Option<StringsFile>,   // .STRINGS
    pub dlstrings: Option<StringsFile>, // .DLSTRINGS
    pub ilstrings: Option<StringsFile>, // .ILSTRINGS
    pub codepage_table: Option<CodepageTable>,
}

impl StringsFiles {
    /// 从目录加载所有 strings 文件(使用默认 UTF-8 编码)
    /// 如果磁盘文件不存在，会尝试从同目录下的 BSA 归档中提取
    pub fn load_from_dir<P: AsRef<Path>>(dir: P, base_name: &str) -> Self {
        let dir = dir.as_ref();
        let strings = Self::try_load(dir, base_name, "english", "STRINGS");
        let dlstrings = Self::try_load(dir, base_name, "english", "DLSTRINGS");
        let ilstrings = Self::try_load(dir, base_name, "english", "ILSTRINGS");

        StringsFiles {
            strings,
            dlstrings,
            ilstrings,
            codepage_table: None,
        }
    }

    /// 从目录加载所有 strings 文件，使用 codepage 配置表
    pub fn load_from_dir_with_codepage<P: AsRef<Path>>(
        dir: P,
        base_name: &str,
        table: &CodepageTable,
    ) -> Self {
        let dir = dir.as_ref();

        let strings = Self::try_load_with_table(dir, base_name, "english", "STRINGS", table);
        let dlstrings = Self::try_load_with_table(dir, base_name, "english", "DLSTRINGS", table);
        let ilstrings = Self::try_load_with_table(dir, base_name, "english", "ILSTRINGS", table);

        StringsFiles {
            strings,
            dlstrings,
            ilstrings,
            codepage_table: Some(table.clone()),
        }
    }

    /// 从目录加载 strings 文件，使用指定的语言和 codepage 配置
    pub fn load_from_dir_with_language<P: AsRef<Path>>(
        dir: P,
        base_name: &str,
        language: &str,
        table: &CodepageTable,
    ) -> Self {
        let dir = dir.as_ref();

        let strings = Self::try_load_with_table(dir, base_name, language, "STRINGS", table);
        let dlstrings = Self::try_load_with_table(dir, base_name, language, "DLSTRINGS", table);
        let ilstrings = Self::try_load_with_table(dir, base_name, language, "ILSTRINGS", table);

        StringsFiles {
            strings,
            dlstrings,
            ilstrings,
            codepage_table: Some(table.clone()),
        }
    }

    fn try_load(dir: &Path, base_name: &str, language: &str, ext: &str) -> Option<StringsFile> {
        let filename = format!("{}_{}.{}", base_name, language, ext);
        let path = dir.join(&filename);

        // 1. 优先尝试从磁盘直接加载
        let format = StringsFile::detect_format(path.as_path());
        if let Ok(sf) = StringsFile::load_with_format(&path, format) {
            return Some(sf);
        }

        // 2. 磁盘文件不存在，尝试从 BSA 归档中提取
        Self::try_load_from_bsa(dir, &filename, format)
    }

    fn try_load_with_table(
        dir: &Path,
        base_name: &str,
        language: &str,
        ext: &str,
        table: &CodepageTable,
    ) -> Option<StringsFile> {
        let filename = format!("{}_{}.{}", base_name, language, ext);
        let path = dir.join(&filename);

        // 1. 优先尝试从磁盘直接加载
        let format = StringsFile::detect_format(path.as_path());
        if let Ok(sf) = StringsFile::load_with_codepage_table(&path, table) {
            return Some(sf);
        }

        // 2. 磁盘文件不存在，尝试从 BSA 归档中提取
        let codepage = table.get_for_filename(&filename);
        Self::try_load_from_bsa_with_codepage(dir, &filename, format, codepage)
    }

    fn try_load_from_bsa(dir: &Path, filename: &str, format: StringsFormat) -> Option<StringsFile> {
        use std::ffi::OsStr;

        let archive_path_in_bsa = format!("strings/{}", filename.to_lowercase());

        if let Ok(entries) = std::fs::read_dir(dir) {
            // 1. 先尝试 BSA 文件（Skyrim/Skyrim SE）
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(OsStr::to_str) == Some("bsa") {
                    if let Ok(bsa) = crate::bsa::BsaArchive::open(&path) {
                        if let Ok(data) = bsa.extract_file(&archive_path_in_bsa) {
                            return StringsFile::load_from_bytes(
                                &data,
                                format,
                                crate::strings::CodepageConfig::utf8(),
                            )
                            .ok();
                        }
                    }
                }
            }

            // 2. 再尝试 BA2 文件（Fallout 4/76/Starfield）
            for entry in std::fs::read_dir(dir).ok()?.flatten() {
                let path = entry.path();
                if path.extension().and_then(OsStr::to_str) == Some("ba2") {
                    if let Ok(ba2) = crate::ba2::Ba2Archive::open(&path) {
                        if let Ok(data) = ba2.extract_file(&archive_path_in_bsa) {
                            return StringsFile::load_from_bytes(
                                &data,
                                format,
                                crate::strings::CodepageConfig::utf8(),
                            )
                            .ok();
                        }
                    }
                }
            }
        }
        None
    }

    fn try_load_from_bsa_with_codepage(
        dir: &Path,
        filename: &str,
        format: StringsFormat,
        codepage: crate::strings::CodepageConfig,
    ) -> Option<StringsFile> {
        use crate::bsa::BsaArchive;
        use std::ffi::OsStr;

        let bsa_path_in_archive = format!("strings/{}", filename.to_lowercase());

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(OsStr::to_str) == Some("bsa") {
                    if let Ok(bsa) = BsaArchive::open(&path) {
                        if let Ok(data) = bsa.extract_file(&bsa_path_in_archive) {
                            return StringsFile::load_from_bytes(&data, format, codepage).ok();
                        }
                    }
                }
            }
        }
        None
    }

    /// 根据 list_index 查找字符串
    pub fn get(&self, list_index: u8, id: u32) -> Option<&String> {
        match list_index {
            0 => self.strings.as_ref()?.get(id),
            1 => self.dlstrings.as_ref()?.get(id),
            2 => self.ilstrings.as_ref()?.get(id),
            _ => None,
        }
    }

    /// 返回已加载的文件数量
    pub fn loaded_count(&self) -> usize {
        let mut count = 0;
        if self.strings.is_some() {
            count += 1;
        }
        if self.dlstrings.is_some() {
            count += 1;
        }
        if self.ilstrings.is_some() {
            count += 1;
        }
        count
    }
}

/// 解析 _recorddefs.txt 格式的字段定义
///
/// 原始 Delphi xTranslator 使用的定义格式：
/// `Def_:FIELD=RECORD=LIST[*|?][-procN]`
///
/// 各部分说明：
/// - FIELD  : 字段签名(4字符，如 FULL, NAM1, DESC)
/// - RECORD : 记录类型(4字符，如 ****=通配, INFO, QUST)
/// - LIST   : Strings 文件索引(0,1,2)+ 可选标记
/// - `*`    : not-null 标记 (字符串不能为空)
/// - `?`    : ignored 标记 (此定义应被忽略)
/// - `-procN` : 内部处理程序标记(当前实现中忽略)
///
/// 示例：
/// - `Def_:FULL=****=0`    → 所有记录的 FULL 字段，使用 .STRINGS
/// - `Def_:NAM1=INFO=2*`   → INFO 记录的 NAM1 字段，使用 .ILSTRINGS，不能为空
/// - `Def_:DATA=GMST=0-proc1` → GMST 记录的 DATA 字段(实际被过滤)
///
/// # 参数
/// * `content` - _recorddefs.txt 文件内容
///
/// # 返回
/// 可翻译字段定义列表
pub fn parse_record_defs(content: &str) -> Vec<TranslatableField> {
    let mut defs = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        // 只处理以 "Def_:" 开头的行
        if !line.starts_with("Def_:") {
            continue;
        }
        // 格式：Def_:FIELD=RECORD=LIST[*|?][-procN]
        // 例如：Def_:FULL=****=0
        let def_part = &line[5..]; // 移除 "Def_:"
        let parts: Vec<&str> = def_part.split('=').collect();
        if parts.len() < 3 {
            continue; // 格式错误：跳过该行
        }

        // 解析字段签名(4字节 ASCII)
        let field_sig = {
            let mut sig = [0u8; 4];
            let bytes = parts[0].as_bytes();
            for (i, &b) in bytes.iter().take(4).enumerate() {
                sig[i] = b;
            }
            sig
        };
        // 解析记录类型签名(4字节 ASCII)
        let record_sig = {
            let mut sig = [0u8; 4];
            let bytes = parts[1].as_bytes();
            for (i, &b) in bytes.iter().take(4).enumerate() {
                sig[i] = b;
            }
            sig
        };

        // 解析 LIST 部分(可能包含 *、?、-proc 等标记)
        let list_str = parts[2];
        let mut not_null = false;
        let mut ignored = false;

        // 检查 ignored 标记(? 表示此定义应被忽略)
        if list_str.find('?').is_some() {
            ignored = true;
        }
        // 检查 not-null 标记(* 表示字符串不能为空)
        if list_str.find('*').is_some() {
            not_null = true;
        }

        // 提取 list_index：只看首个数字(0/1/2)，忽略后续标记(如 -procN)。
        let list_index = list_str
            .chars()
            .next()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(0) as u8;

        defs.push(TranslatableField {
            record_sig,
            field_sig,
            list_index,
            not_null,
            ignored,
        });
    }
    defs
}

/// 根据 GameId 获取 Data 目录下的游戏子目录名
pub fn game_data_subdir(game: GameId) -> &'static str {
    match game {
        GameId::Skyrim => "Skyrim",
        GameId::SkyrimSE => "SkyrimSE",
        GameId::Fallout4 => "Fallout4",
        GameId::FalloutNV => "FalloutNV",
        GameId::Fallout76 => "Fallout76",
        GameId::Starfield => "Starfield",
    }
}

/// 从 Data 目录加载指定游戏的 record_defs
pub fn load_game_record_defs(
    data_dir: &Path,
    game: GameId,
) -> std::io::Result<Vec<TranslatableField>> {
    let subdir = game_data_subdir(game);
    let path = data_dir.join(subdir).join("_recorddefs.txt");
    let content = std::fs::read_to_string(&path)?;
    Ok(parse_record_defs(&content))
}

/// 简单的 ESP 解析器 PoC
pub struct EspParser {
    pub record_defs: Vec<TranslatableField>,
    pub strings: Vec<SkyString>,
    pub strings_files: StringsFiles,
    pub compressed_records: u32,
    current_parent_form_id: u32,
    progress_callback: Option<Box<dyn Fn(u64) + Send>>,
    /// Pre-built HashMap for O(1) field def lookup: (record_sig, field_sig) -> index
    def_map: HashMap<([u8; 4], [u8; 4]), usize>,
    build_search_index: bool,
    /// Whether to build the full record tree (ESP mode).
    esp_mode: bool,
    /// The in-memory record tree, populated when esp_mode is true.
    pub record_tree: Vec<EspGrup>,
    /// TES4 header data, stored when esp_mode is true.
    pub tes4_header: Option<Tes4Header>,
}

impl EspParser {
    fn build_def_map(defs: &[TranslatableField]) -> HashMap<([u8; 4], [u8; 4]), usize> {
        let mut map = HashMap::new();
        for (i, def) in defs.iter().enumerate() {
            if !def.ignored {
                map.insert((def.record_sig, def.field_sig), i);
            }
        }
        map
    }

    pub fn new() -> Self {
        let default_defs = include_str!("../esp_default_defs.txt");
        let record_defs = parse_record_defs(default_defs);
        let def_map = Self::build_def_map(&record_defs);
        Self {
            record_defs,
            strings: Vec::new(),
            strings_files: StringsFiles::default(),
            compressed_records: 0,
            current_parent_form_id: 0,
            progress_callback: None,
            def_map,
            build_search_index: true,
            esp_mode: false,
            record_tree: Vec::new(),
            tes4_header: None,
        }
    }

    pub fn with_defs(defs: Vec<TranslatableField>) -> Self {
        let def_map = Self::build_def_map(&defs);
        Self {
            record_defs: defs,
            strings: Vec::new(),
            strings_files: StringsFiles::default(),
            compressed_records: 0,
            current_parent_form_id: 0,
            progress_callback: None,
            def_map,
            build_search_index: true,
            esp_mode: false,
            record_tree: Vec::new(),
            tes4_header: None,
        }
    }

    /// 使用指定游戏的完整 record_defs 创建解析器
    pub fn with_game(data_dir: &Path, game: GameId) -> std::io::Result<Self> {
        let defs = load_game_record_defs(data_dir, game)?;
        Ok(Self::with_defs(defs))
    }

    /// Disable normalization/word indexes when callers only need raw parsed strings.
    pub fn set_build_search_index(&mut self, build_search_index: bool) {
        self.build_search_index = build_search_index;
    }

    /// Enable ESP mode — triggers full record tree build on next parse.
    ///
    /// When ESP mode is active, the parser retains the full in-memory record tree
    /// alongside the extracted strings, enabling write-back to the ESP file.
    pub fn enable_esp_mode(&mut self) {
        self.esp_mode = true;
    }

    /// Check if ESP mode is enabled.
    pub fn is_esp_mode(&self) -> bool {
        self.esp_mode
    }

    /// Build an EspFile from the current state (TES4 header + record tree).
    ///
    /// Returns None if ESP mode was not active during parsing.
    pub fn build_esp_file(&self) -> Option<EspFile> {
        let tes4 = self.tes4_header.clone()?;
        Some(EspFile {
            tes4,
            top_level_grups: self.record_tree.clone(),
        })
    }

    /// 设置进度回调函数
    ///
    /// 回调函数接收当前已处理的字节数
    pub fn set_progress_callback<F>(&mut self, callback: F)
    where
        F: Fn(u64) + Send + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
    }

    /// 报告进度
    fn report_progress(&self, bytes_processed: u64) {
        if let Some(ref cb) = self.progress_callback {
            cb(bytes_processed);
        }
    }

    /// 加载 Strings 文件(.STRINGS/.DLSTRINGS/.ILSTRINGS)
    ///
    /// 根据基础文件名自动加载三种格式的字符串文件：
    /// - {base_name}_english.STRINGS
    /// - {base_name}_english.DLSTRINGS
    /// - {base_name}_english.ILSTRINGS
    ///
    /// # 参数
    /// * `dir` - 字符串文件所在目录
    /// * `base_name` - 基础文件名(如 "Skyrim" 会加载 Skyrim_english.STRINGS 等)
    pub fn load_strings_files<P: AsRef<Path>>(&mut self, dir: P, base_name: &str) {
        self.strings_files = StringsFiles::load_from_dir(dir, base_name);
    }

    /// 设置已加载的 Strings 文件集合
    ///
    /// 当外部已经加载好字符串文件时使用此方法
    pub fn set_strings_files(&mut self, files: StringsFiles) {
        self.strings_files = files;
    }

    /// 解析 ESP/ESM 文件
    pub fn parse<R: Read>(&mut self, reader: &mut R) -> Result<()> {
        self.strings.clear();
        self.record_tree.clear();
        self.tes4_header = None;

        // 先读取 TES4 头记录(插件主头)。
        let tes4_generic = GenericHeader::read_from(reader)?;
        if !tes4_generic.is_tes4() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Expected TES4 header",
            ));
        }

        // 读取 TES4 RecordHeaderData(16 字节)。
        let tes4_record_header = RecordHeaderData::read_from(reader)?;

        // 读取 TES4 字段体(注意：dsize 不包含 RecordHeaderData 本身)。
        let mut tes4_data = vec![0u8; tes4_generic.dsize as usize];
        reader.read_exact(&mut tes4_data)?;

        // Store TES4 header if in ESP mode
        if self.esp_mode {
            self.tes4_header = Some(Tes4Header {
                generic: tes4_generic.clone(),
                record_header_data: tes4_record_header.clone(),
                field_data: tes4_data.clone(),
            });
        }

        // 直接解析 TES4 字段；RecordHeaderData 已在上方消费。
        self.parse_record_fields_direct(b"TES4", 0, &tes4_data)?;

        // 读取后续全部记录/组数据并进入顶层遍历。
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        let mut cursor = Cursor::new(&buf);

        let mut grup_count = 0u32;
        let mut record_count = 0u32;
        let total_bytes = buf.len() as u64;
        let mut last_reported_percentage = 0u8;

        while cursor.position() < buf.len() as u64 {
            let pos = cursor.position();

            // 每处理 1MB 或百分比变化时报告进度
            let current_percentage = ((pos as f64 / total_bytes as f64) * 100.0) as u8;
            if current_percentage != last_reported_percentage && current_percentage % 5 == 0 {
                self.report_progress(pos);
                last_reported_percentage = current_percentage;
            }

            match self.parse_top_level_debug(&mut cursor, &mut grup_count, &mut record_count) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    eprintln!("Error at byte {}: {:?}", pos, e);
                    return Err(e);
                }
            }
        }

        // 报告完成
        self.report_progress(total_bytes);

        Ok(())
    }

    #[allow(dead_code)]
    fn parse_top_level<R: Read>(&mut self, reader: &mut R) -> Result<()> {
        self.parse_top_level_debug(reader, &mut 0, &mut 0)
    }

    fn parse_top_level_debug<R: Read>(
        &mut self,
        reader: &mut R,
        grup_count: &mut u32,
        record_count: &mut u32,
    ) -> Result<()> {
        let header = match GenericHeader::read_from(reader) {
            Ok(h) => h,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };

        if header.is_grup() {
            *grup_count += 1;
            // 读取 GRUP 专用头(16 字节)。
            let grup_header = GrupHeader::read_from(reader)?;
            // GRUP 结构：GenericHeader(8) + GrupHeader(16) + payload
            // 注意：GRUP 的 dsize 包含自身头部，因此 payload = dsize - 24。
            let grup_data_size = if header.dsize >= 24 {
                header.dsize as usize - 24
            } else {
                0
            };

            let mut grup = if self.esp_mode {
                Some(EspGrup {
                    header: header.clone(),
                    grup_header: grup_header.clone(),
                    records: Vec::new(),
                    children: Vec::new(),
                })
            } else {
                None
            };

            if grup_data_size > 0 {
                let mut grup_data = vec![0u8; grup_data_size];
                reader.read_exact(&mut grup_data)?;

                let mut cursor = Cursor::new(&grup_data);
                while cursor.position() < grup_data.len() as u64 {
                    match self.parse_record_debug_for_tree(&mut cursor, record_count, &mut grup) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                        Err(e) => {
                            eprintln!(
                                "Warning: error parsing nested record at byte {}: {:?}",
                                cursor.position(),
                                e
                            );
                            break;
                        }
                    }
                }
            }

            if let Some(grup) = grup {
                self.record_tree.push(grup);
            }
        } else {
            // 顶层普通记录(非 GRUP)：这里只消费字节，不在此层提取字段。
            let mut data = vec![0u8; header.dsize as usize];
            reader.read_exact(&mut data)?;
        }

        Ok(())
    }

    /// Parse a record (or nested GRUP) and optionally build the record tree.
    ///
    /// When `grup` is Some, parsed records are added to the GRUP's records vector.
    /// Translatable strings are extracted and SkyString.field_ref is set.
    fn parse_record_debug_for_tree<R: Read>(
        &mut self,
        reader: &mut R,
        record_count: &mut u32,
        parent_grup: &mut Option<EspGrup>,
    ) -> Result<()> {
        let header = match GenericHeader::read_from(reader) {
            Ok(h) => h,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };

        if header.is_grup() {
            // Nested GRUP
            *record_count = record_count.saturating_sub(1);
            let grup_header = GrupHeader::read_from(reader)?;
            let saved_parent = self.current_parent_form_id;
            if grup_header.s_type != 0 {
                self.current_parent_form_id = grup_header.s_type;
            }

            let grup_data_size = if header.dsize >= 24 {
                header.dsize as usize - 24
            } else {
                0
            };

            let mut child_grup = if self.esp_mode {
                Some(EspGrup {
                    header: header.clone(),
                    grup_header: grup_header.clone(),
                    records: Vec::new(),
                    children: Vec::new(),
                })
            } else {
                None
            };

            if grup_data_size > 0 {
                let mut grup_data = vec![0u8; grup_data_size];
                reader.read_exact(&mut grup_data)?;

                let mut cursor = Cursor::new(&grup_data);
                while cursor.position() < grup_data.len() as u64 {
                    match self.parse_record_debug_for_tree(&mut cursor, record_count, &mut child_grup) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                        Err(e) => {
                            eprintln!("Warning: error parsing nested record: {:?}", e);
                            break;
                        }
                    }
                }
            }

            self.current_parent_form_id = saved_parent;

            if let (Some(child), Some(parent)) = (child_grup, parent_grup.as_mut()) {
                parent.children.push(child);
            }
            return Ok(());
        }

        // Normal record
        *record_count += 1;
        let record_header_data = RecordHeaderData::read_from(reader)?;
        let data_size = header.dsize as usize;
        let mut record_data = vec![0u8; data_size];
        reader.read_exact(&mut record_data)?;

        let is_compressed = record_header_data.is_compressed();
        let form_id = record_header_data.form_id;

        if is_compressed {
            self.compressed_records += 1;
        }

        // Parse fields for the tree
        let (fields, decompressed_data, raw) = if is_compressed {
            match decompress_bethesda_record(&record_data) {
                Ok(decompressed) => {
                    let fields = EspField::parse_fields(&decompressed).unwrap_or_default();
                    (fields, decompressed, false)
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to decompress record {:?}: {}",
                        header.name, e
                    );
                    // Treat as raw
                    (Vec::new(), record_data.clone(), true)
                }
            }
        } else {
            let fields = EspField::parse_fields(&record_data).unwrap_or_default();
            (fields, record_data.clone(), false)
        };

        // Extract editor ID from fields
        let editor_id = fields
            .iter()
            .find(|f| f.header.name == *b"EDID")
            .and_then(|f| {
                let len = f.buffer.iter().position(|&b| b == 0).unwrap_or(f.buffer.len());
                String::from_utf8(f.buffer[..len].to_vec()).ok()
            });

        // Extract translatable strings and set field_ref
        if !raw {
            self.extract_strings_from_fields(
                &header.name,
                form_id,
                &fields,
                &decompressed_data,
                editor_id.as_deref(),
                parent_grup.as_ref().map(|g| g.records.len()),
            );
        }

        // Build EspRecord
        let esp_record = EspRecord {
            header: header.clone(),
            record_header_data,
            fields,
            compressed: is_compressed,
            raw,
            form_id,
            editor_id,
            original_raw_data: if raw { record_data } else { decompressed_data },
        };

        if let Some(grup) = parent_grup.as_mut() {
            grup.records.push(esp_record);
        }

        Ok(())
    }

    /// Extract translatable strings from parsed fields and set field_ref.
    fn extract_strings_from_fields(
        &mut self,
        record_sig: &[u8; 4],
        form_id: u32,
        fields: &[EspField],
        _record_data: &[u8],
        editor_id: Option<&str>,
        _record_index_in_grup: Option<usize>,
    ) {
        let mut field_index = 0u16;

        for (field_vec_idx, field) in fields.iter().enumerate() {
            if field.is_size_xxxx {
                continue;
            }

            // Check if this is a translatable field
            if let Some(def) = self.find_def(record_sig, &field.header.name) {
                // GMST:DATA filter
                if record_sig == b"GMST" && &field.header.name == b"DATA" {
                    let is_string_gmst = editor_id.map(|e| e.starts_with('s')).unwrap_or(false);
                    if !is_string_gmst {
                        field_index += 1;
                        continue;
                    }
                }

                // Extract string ID from field buffer
                if field.buffer.len() >= 4 {
                    let string_id = u32::from_le_bytes([
                        field.buffer[0],
                        field.buffer[1],
                        field.buffer[2],
                        field.buffer[3],
                    ]);

                    let source_text = self
                        .strings_files
                        .get(def.list_index, string_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<ID:{}>", string_id));

                    if !source_text.is_empty() {
                        let mut sk = if self.build_search_index {
                            SkyString::new(
                                self.strings.len() as u32,
                                source_text,
                                String::new(),
                                *record_sig,
                                field.header.name,
                            )
                        } else {
                            SkyString::new_without_search_index(
                                self.strings.len() as u32,
                                source_text,
                                String::new(),
                                *record_sig,
                                field.header.name,
                            )
                        };

                        sk.esp_ptr = EspPointer {
                            str_id: string_id as i32,
                            form_id,
                            record_sig: *record_sig,
                            field_sig: field.header.name,
                            index: field_index,
                            index_max: 1,
                            edid_hash: editor_id.map_or(0, |s| string_hash(s)),
                        };

                        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
                        sk.list_index = def.list_index;
                        sk.parent_form_id = self.current_parent_form_id;
                        // Set field_ref for ESP mode write-back
                        sk.field_ref = Some(field_vec_idx);

                        self.strings.push(sk);
                    }
                }
            }

            // Handle VMAD fields
            if &field.header.name == b"VMAD" && !field.buffer.is_empty() {
                self.parse_vmad_strings(record_sig, form_id, &field.buffer, field_index);
            }

            field_index += 1;
        }
    }

    fn parse_record_debug<R: Read>(
        &mut self,
        reader: &mut R,
        record_count: &mut u32,
    ) -> Result<()> {
        let header = match GenericHeader::read_from(reader) {
            Ok(h) => h,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };

        if header.is_grup() {
            // 嵌套 GRUP：递归解析其中的子记录。
            // GRUP 本身不计入 record；空计数时避免 unsigned underflow。
            *record_count = record_count.saturating_sub(1);
            let grup_header = GrupHeader::read_from(reader)?;
            // s_type contains the parent FormID for child GRUPs or record type for type GRUPs
            let saved_parent = self.current_parent_form_id;
            if grup_header.s_type != 0 {
                self.current_parent_form_id = grup_header.s_type;
            }
            // GRUP 结构：GenericHeader(8) + GrupHeader(16) + payload
            // dsize 包含头部，因此 payload = dsize - 24。
            let grup_data_size = if header.dsize >= 24 {
                header.dsize as usize - 24
            } else {
                0
            };

            if grup_data_size > 0 {
                let mut grup_data = vec![0u8; grup_data_size];
                reader.read_exact(&mut grup_data)?;

                let mut cursor = Cursor::new(&grup_data);
                while cursor.position() < grup_data.len() as u64 {
                    match self.parse_record_debug(&mut cursor, record_count) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                        Err(e) => {
                            eprintln!("Warning: error parsing nested record: {:?}", e);
                            break;
                        }
                    }
                }
            }
            self.current_parent_form_id = saved_parent;
            return Ok(());
        }

        let record_header_data = RecordHeaderData::read_from(reader)?;
        // 普通 Record：dsize 仅表示字段区大小，不包含 RecordHeaderData。
        let data_size = header.dsize as usize;
        let mut record_data = vec![0u8; data_size];
        reader.read_exact(&mut record_data)?;

        // 压缩记录：先解压再解析字段(否则字段偏移全部失效)。
        if record_header_data.is_compressed() {
            self.compressed_records += 1;
            match decompress_bethesda_record(&record_data) {
                Ok(decompressed) => {
                    // 解压后数据与普通记录字段布局一致，走同一套字段解析逻辑。
                    self.parse_record_fields_with_id(
                        &header.name,
                        record_header_data.form_id,
                        &decompressed,
                    )?;
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to decompress record {:?}: {}",
                        header.name, e
                    );
                }
            }
            return Ok(()); // 压缩记录处理完成
        }

        // 非压缩记录：直接按字段流解析并提取字符串。
        self.parse_record_fields_with_id(&header.name, record_header_data.form_id, &record_data)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn parse_record_fields(&mut self, record_sig: &[u8; 4], data: &[u8]) -> Result<()> {
        self.parse_record_fields_with_id(record_sig, 0, data)
    }

    fn parse_record_fields_with_id(
        &mut self,
        record_sig: &[u8; 4],
        form_id: u32,
        data: &[u8],
    ) -> Result<()> {
        // RecordHeaderData 已在上层读取，这里直接进入字段解析。
        self.parse_record_fields_direct(record_sig, form_id, data)
    }

    fn parse_record_fields_direct(
        &mut self,
        record_sig: &[u8; 4],
        form_id: u32,
        data: &[u8],
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let mut cursor = Cursor::new(data);
        let mut next_field_size: u32 = 0;
        let mut field_index = 0u16;
        let mut edid: Option<String> = None;

        while cursor.position() < data.len() as u64 {
            let field_header = match FieldHeader::read_from(&mut cursor) {
                Ok(h) => h,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            let data_size = if next_field_size > 0 {
                let size = next_field_size as u16;
                next_field_size = 0;
                size
            } else {
                field_header.dsize
            };

            // 边界保护：字段声明大小超过剩余字节时，终止当前记录解析。
            let remaining = data.len() as u64 - cursor.position();
            if data_size as u64 > remaining {
                // 记录体可能损坏：不再继续读取，避免产生级联误解析。
                break;
            }

            let mut field_data = vec![0u8; data_size as usize];
            cursor.read_exact(&mut field_data)?;

            if field_header.is_xxxx() {
                // XXXX 是 Bethesda 的“扩展长度标记”：
                // 本字段内容携带“下一字段”的真实 32 位长度。
                if field_data.len() >= 4 {
                    next_field_size = u32::from_le_bytes([
                        field_data[0],
                        field_data[1],
                        field_data[2],
                        field_data[3],
                    ]);
                }
                continue;
            }

            // 提取 EDID(null-terminated ASCII)，供后续 GMST:DATA 类型判断使用。
            if &field_header.name == b"EDID" && !field_data.is_empty() {
                let len = field_data
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(field_data.len());
                edid = Some(String::from_utf8_lossy(&field_data[..len]).to_string());
            }

            // 检查是否是可翻译字段(根据 record_defs.txt 定义)
            if let Some(def) = self.find_def(record_sig, &field_header.name) {
                // GMST:DATA 过滤：只保留字符串型(EDID 以 's' 开头)，跳过数值型(f/i/b)。
                if record_sig == b"GMST" && &field_header.name == b"DATA" {
                    let is_string_gmst = edid.as_ref().map(|e| e.starts_with('s')).unwrap_or(false);
                    if !is_string_gmst {
                        continue;
                    }
                }

                // 可翻译字段约定：字段体前 4 字节是小端字符串 ID。
                // 该 ID 用于到 STRINGS/DLSTRINGS/ILSTRINGS 中反查文本。
                if field_data.len() >= 4 {
                    let string_id = u32::from_le_bytes([
                        field_data[0],
                        field_data[1],
                        field_data[2],
                        field_data[3],
                    ]);

                    // 查表失败时保留占位文本，便于定位缺失映射。
                    let source_text = self
                        .strings_files
                        .get(def.list_index, string_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<ID:{}>", string_id));

                    if !source_text.is_empty() {
                        // 构建一条可编辑字符串记录。
                        let mut sk = if self.build_search_index {
                            SkyString::new(
                                self.strings.len() as u32, // 内部 ID
                                source_text,               // 源字符串
                                String::new(),             // 翻译(初始为空)
                                *record_sig,               // 记录类型
                                field_header.name,         // 字段类型
                            )
                        } else {
                            SkyString::new_without_search_index(
                                self.strings.len() as u32, // 内部 ID
                                source_text,               // 源字符串
                                String::new(),             // 翻译(初始为空)
                                *record_sig,               // 记录类型
                                field_header.name,         // 字段类型
                            )
                        };

                        // 填充 ESP 指针，用于后续 SST/XML 精确匹配与写回。
                        sk.esp_ptr = EspPointer {
                            str_id: string_id as i32,     // 实际的字符串 ID(关键：用于 XML 匹配)
                            form_id,                      // 记录的 FormID
                            record_sig: *record_sig,      // 记录类型
                            field_sig: field_header.name, // 字段类型
                            index: field_index,           // 字段索引
                            index_max: 1,                 // 字段总数
                            edid_hash: edid.as_ref().map_or(0, |s| string_hash(s)), // Editor ID 的 FNV-1a 哈希
                        };

                        // 初始状态：有源文、无译文 => 未完成翻译。
                        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
                        sk.list_index = def.list_index;
                        sk.parent_form_id = self.current_parent_form_id;

                        self.strings.push(sk);
                    }
                }
            }

            // 处理 VMAD 字段中的脚本字符串
            if &field_header.name == b"VMAD" && !field_data.is_empty() {
                self.parse_vmad_strings(record_sig, form_id, &field_data, field_index);
            }

            field_index += 1;
        }

        Ok(())
    }

    /// 解析 VMAD 字段中的脚本字符串
    fn parse_vmad_strings(
        &mut self,
        record_sig: &[u8; 4],
        form_id: u32,
        data: &[u8],
        field_index: u16,
    ) {
        use crate::types::esp_pointer::string_hash;

        // VMAD 字段前 2 字节是版本号 (i16 LE)
        let vmad_version = if data.len() >= 2 {
            i16::from_le_bytes([data[0], data[1]])
        } else {
            return; // 数据太短，无法解析
        };

        let decoder = VmadDecoder::new(data, vmad_version);
        let vmad_strings = decoder.decode();

        for vmad_str in vmad_strings {
            if vmad_str.value.is_empty() {
                continue;
            }

            let mut sk = if self.build_search_index {
                SkyString::new(
                    self.strings.len() as u32,
                    vmad_str.value.clone(),
                    String::new(),
                    *record_sig,
                    *b"VMAD",
                )
            } else {
                SkyString::new_without_search_index(
                    self.strings.len() as u32,
                    vmad_str.value.clone(),
                    String::new(),
                    *record_sig,
                    *b"VMAD",
                )
            };

            // VMAD 信息编码到 esp_ptr:
            // - str_id: 负偏移量，标识 VMAD 字符串（后续写回时使用）
            // - index: field_index（原始字段索引）
            // - index_max: vmad_length（字符串长度，用于写回验证）
            // - edid_hash: script_name + prop_name 的组合哈希（用于标识）
            let script_prop_key = format!("{}\0{}", vmad_str.script_name, vmad_str.prop_name);
            sk.esp_ptr = EspPointer {
                str_id: -(vmad_str.offset as i32),
                form_id,
                record_sig: *record_sig,
                field_sig: *b"VMAD",
                index: field_index,
                index_max: vmad_str.length as u16,
                edid_hash: string_hash(&script_prop_key),
            };

            sk.internal_params.set(
                crate::types::params::SkyStringInternalParams::IS_VMAD_STRING,
                true,
            );
            sk.list_index = 0;
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
            sk.parent_form_id = self.current_parent_form_id;

            self.strings.push(sk);
        }
    }

    fn find_def(&self, record_sig: &[u8; 4], field_sig: &[u8; 4]) -> Option<&TranslatableField> {
        // O(1) HashMap lookup, with wildcard record_sig fallback
        let key = (*record_sig, *field_sig);
        if let Some(&idx) = self.def_map.get(&key) {
            return Some(&self.record_defs[idx]);
        }
        let wildcard_key = (*b"****", *field_sig);
        if let Some(&idx) = self.def_map.get(&wildcard_key) {
            return Some(&self.record_defs[idx]);
        }
        None
    }
}

impl Default for EspParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_record_defs() {
        let content = r#"
Def_:FULL=****=0
Def_:DESC=****=1
Def_:NAM1=INFO=2
Def_:CNAM=QUST=1
"#;
        let defs = parse_record_defs(content);
        assert_eq!(defs.len(), 4);
        assert_eq!(&defs[0].field_sig, b"FULL");
        assert_eq!(&defs[0].record_sig, b"****");
        assert_eq!(defs[0].list_index, 0);
        assert!(!defs[0].not_null);
        assert!(!defs[0].ignored);
    }

    #[test]
    fn test_parse_record_defs_with_markers() {
        let content = r#"
Def_:NAM1=INFO=2*
Def_:DATA=GMST=0-proc1
Def_:FULL=IMAD=0?(ignored)
Def_:CNAM=DOOR=0-proc5
"#;
        let defs = parse_record_defs(content);
        assert_eq!(defs.len(), 4);

        // NAM1=INFO=2* → not_null=true
        assert_eq!(&defs[0].field_sig, b"NAM1");
        assert_eq!(defs[0].list_index, 2);
        assert!(defs[0].not_null);
        assert!(!defs[0].ignored);

        // DATA=GMST=0-proc1 → proc ignored, list=0
        assert_eq!(&defs[1].field_sig, b"DATA");
        assert_eq!(defs[1].list_index, 0);
        assert!(!defs[1].not_null);
        assert!(!defs[1].ignored);

        // FULL=IMAD=0? → ignored=true
        assert_eq!(&defs[2].field_sig, b"FULL");
        assert_eq!(defs[2].list_index, 0);
        assert!(!defs[2].not_null);
        assert!(defs[2].ignored);

        // CNAM=DOOR=0-proc5
        assert_eq!(&defs[3].field_sig, b"CNAM");
        assert_eq!(defs[3].list_index, 0);
    }

    #[test]
    fn test_parse_skyrimse_record_defs() {
        let content = include_str!("../esp_default_defs.txt");
        let defs = parse_record_defs(content);
        // 默认定义应包含至少 20 条
        assert!(
            defs.len() >= 20,
            "Expected at least 20 defs, got {}",
            defs.len()
        );

        // FULL=****=0 应存在
        let full_wildcard = defs
            .iter()
            .find(|d| &d.field_sig == b"FULL" && &d.record_sig == b"****");
        assert!(full_wildcard.is_some());
        assert_eq!(full_wildcard.unwrap().list_index, 0);

        // NAM1=INFO=2* 应有 not_null 标记
        let nam1_info = defs
            .iter()
            .find(|d| &d.field_sig == b"NAM1" && &d.record_sig == b"INFO");
        assert!(nam1_info.is_some());
        assert!(nam1_info.unwrap().not_null);
    }

    #[test]
    fn test_parse_record_debug_empty_grup_does_not_underflow() {
        let mut parser = EspParser::new();
        let mut record_count = 0u32;
        let data = [
            b'G', b'R', b'U', b'P', 24, 0, 0, 0, // GenericHeader
            b'T', b'E', b'S', b'4', // GrupHeader s_ident
            0, 0, 0, 0, // s_type
            0, 0, // s_tstamp
            0, 0, // param1
            0, 0, // param2
            0, 0, // param3
        ];
        let mut cursor = Cursor::new(data);

        parser
            .parse_record_debug(&mut cursor, &mut record_count)
            .expect("empty GRUP should not underflow");

        assert_eq!(record_count, 0);
    }

    #[test]
    fn test_game_data_subdir() {
        assert_eq!(game_data_subdir(GameId::Skyrim), "Skyrim");
        assert_eq!(game_data_subdir(GameId::SkyrimSE), "SkyrimSE");
        assert_eq!(game_data_subdir(GameId::Fallout4), "Fallout4");
        assert_eq!(game_data_subdir(GameId::Starfield), "Starfield");
    }

    #[test]
    fn test_find_def_ignores_ignored() {
        let defs = vec![
            TranslatableField::new(*b"TEST", *b"FULL", 0),
            TranslatableField {
                record_sig: *b"TEST",
                field_sig: *b"DESC",
                list_index: 1,
                not_null: false,
                ignored: true,
            },
        ];
        let parser = EspParser::with_defs(defs);
        // FULL 应该能找到
        assert!(parser.find_def(b"TEST", b"FULL").is_some());
        // DESC 被标记为 ignored，应该找不到
        assert!(parser.find_def(b"TEST", b"DESC").is_none());
    }

    #[test]
    fn test_load_strings_from_bsa_fallback() {
        // Skyrim SE strings files are inside Skyrim - Interface.bsa
        // This test verifies BSA fallback when standalone files don't exist
        let data_dir = Path::new(r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data");
        if !data_dir.exists() {
            return; // Skip if Skyrim SE is not installed
        }

        let sf = StringsFiles::load_from_dir(data_dir, "skyrim");

        // At least one strings file should be loaded (from BSA)
        let loaded = sf.loaded_count();
        println!("Loaded {} strings files from BSA fallback", loaded);
        assert!(
            loaded > 0,
            "Expected at least one strings file loaded from BSA, got {}",
            loaded
        );

        // Verify we can look up a known string ID
        // Skyrim.esm has many GMST records that reference strings by ID
        if let Some(s) = sf.get(0, 1) {
            println!("String ID 1 (list 0): {}", s);
        }
    }
}
