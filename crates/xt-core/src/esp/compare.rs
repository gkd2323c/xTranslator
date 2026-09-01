//! ESP 文件对比 —— 构建两个插件文件之间的字符串对
//!
//! 对比两个 ESP/ESM 文件并生成字符串对的映射：
//! - identical: 字符串完全匹配
//! - modified: 匹配位置相同但文本不同
//! - added: 存在于新文件中但不存在于旧文件中
//! - removed: 存在于旧文件中但不存在于新文件中
//!
//! 该对比遵循原版 Delphi xTranslator ESPCompare 的路径：
//! 流式读取记录，仅保留字符串字段，并尽可能通过 FormID + 字段出现次数进行匹配。
//! 缺少 FormID 的合成/无 FormID 数据将回退到现有测试中使用的传统 (str_id, record_sig, field_sig) 三元组。
//!
//! 使用轻量级的对比专用缓存（避免存储带有规范化/哈希等的完整 SkyString），以实现快速反序列化。

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Cursor, ErrorKind, Read, Result as IoResult};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::esp::header::{FieldHeader, GenericHeader, GrupHeader, RecordHeaderData};
use crate::esp::parser::{
    decompress_bethesda_record, load_game_record_defs, parse_record_defs, StringsFiles,
    TranslatableField,
};
use crate::types::esp_pointer::{split_form_id_identity, string_hash};
use crate::types::game_id::GameId;
use crate::types::sky_string::SkyString;
use crate::vmad::VmadDecoder;

/// 用于对比的轻量级条目。
///
/// 仅存储 ESPCompare 所需的位置和源文本。
/// 提取器直接从 ESP 字段中创建这些条目，而不是分配完整的 SkyString 值及其搜索索引。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompareEntry {
    pub id: u32,
    pub str_id: i32,
    pub source: String,
    pub record_sig: [u8; 4],
    pub field_sig: [u8; 4],
    pub form_id: u32,
    pub form_owner_hash: u32,
    pub local_form_id: u32,
    pub field_index: u16,
    pub field_index_max: u16,
    pub edid_hash: u32,
    pub source_hash: u32,
}

impl CompareEntry {
    fn from_sky_string(s: &SkyString) -> Self {
        Self {
            id: s.id,
            str_id: s.esp_ptr.str_id,
            source: s.source.clone(),
            record_sig: s.esp_ptr.record_sig,
            field_sig: s.esp_ptr.field_sig,
            form_id: s.esp_ptr.form_id,
            form_owner_hash: 0,
            local_form_id: s.esp_ptr.form_id,
            field_index: s.esp_ptr.index,
            field_index_max: s.esp_ptr.index_max,
            edid_hash: s.esp_ptr.edid_hash,
            source_hash: string_hash(&s.source),
        }
    }
}

/// 两个 ESP 文件之间的对比结果
#[derive(Debug, Clone)]
pub struct EspComparison {
    /// 来自旧 ESP 的所有字符串（按内部 ID）
    pub old_strings: Vec<CompareEntry>,
    /// 来自新 ESP 的所有字符串（按内部 ID）
    pub new_strings: Vec<CompareEntry>,
    /// 映射：匹配条目的新内部 ID -> 旧内部 ID
    pub matched_pairs: HashMap<u32, u32>,
    /// 存在于新文件中但不在旧文件中的字符串（使用 HashSet 进行 O(1) 查找）
    pub added: HashSet<u32>,
    /// 存在于旧文件中但不在新文件中的字符串（使用 HashSet 进行 O(1) 查找）
    pub removed: HashSet<u32>,
    /// 键相同但文本不同的字符串（使用 HashSet 进行 O(1) 查找）
    pub modified: HashSet<u32>,
    /// 映射：已修改条目的新 ID -> 旧 ID（相同键，不同文本）
    pub modified_pairs: HashMap<u32, u32>,
}

/// 用于匹配 ESP 文件之间字符串的键。
///
/// 原版 xTranslator 按纯 FormID 对对比记录进行排序，然后匹配第一个未使用的具有相同字段签名的字符串字段。
/// `field_index` 模拟了这种重复字段的出现情况。当缺失 FormID 时（单元测试、导入的合成集），
/// 我们保持之前的 string-id 三元组行为。
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct StringKey {
    pub str_id: i32,
    pub form_owner_hash: u32,
    pub local_form_id: u32,
    pub record_sig: [u8; 4],
    pub field_sig: [u8; 4],
    pub field_index: u16,
    pub edid_hash: u32,
}

impl StringKey {
    fn from_compare_entry(e: &CompareEntry) -> Self {
        if e.form_id == 0 {
            return Self {
                str_id: e.str_id,
                form_owner_hash: 0,
                local_form_id: 0,
                record_sig: e.record_sig,
                field_sig: e.field_sig,
                field_index: 0,
                edid_hash: 0,
            };
        }

        Self {
            str_id: 0,
            form_owner_hash: e.form_owner_hash,
            local_form_id: if e.form_owner_hash == 0 {
                e.form_id
            } else {
                e.local_form_id
            },
            record_sig: e.record_sig,
            field_sig: e.field_sig,
            field_index: e.field_index,
            edid_hash: if &e.field_sig == b"VMAD" {
                e.edid_hash
            } else {
                0
            },
        }
    }
}

const COMPARE_CACHE_VERSION: u32 = 3;

#[derive(Serialize, Deserialize)]
struct CompareCachePayload {
    version: u32,
    game: String,
    entries: Vec<CompareEntry>,
}

fn compare_cache_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("xTranslator")
            .join("compare_cache")
    } else {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".cache"))
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("xTranslator")
            .join("compare_cache")
    }
}

fn file_hash(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

fn game_cache_key(game: GameId) -> &'static str {
    match game {
        GameId::Skyrim => "skyrim",
        GameId::SkyrimSE => "skyrimse",
        GameId::Fallout4 => "fallout4",
        GameId::FalloutNV => "falloutnv",
        GameId::Fallout76 => "fallout76",
        GameId::Starfield => "starfield",
    }
}

fn cache_path(hash: &str, game: GameId) -> PathBuf {
    compare_cache_dir().join(format!("{}_{}.compare", hash, game_cache_key(game)))
}

/// 从磁盘加载缓存的 CompareEntries（如果可用且有效）
fn load_cached_entries(hash: &str, game: GameId) -> Option<Vec<CompareEntry>> {
    let path = cache_path(hash, game);
    if !path.exists() {
        return None;
    }
    let data = std::fs::read(&path).ok()?;
    let payload: CompareCachePayload = match bincode::deserialize(&data) {
        Ok(payload) => payload,
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            return None;
        }
    };
    if payload.version != COMPARE_CACHE_VERSION || payload.game != game_cache_key(game) {
        let _ = std::fs::remove_file(&path);
        return None;
    }
    Some(payload.entries)
}

/// 将 CompareEntries 写入磁盘以便未来快速加载
fn store_cached_entries(hash: &str, game: GameId, entries: &[CompareEntry]) {
    let dir = compare_cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    let payload = CompareCachePayload {
        version: COMPARE_CACHE_VERSION,
        game: game_cache_key(game).to_string(),
        entries: entries.to_vec(),
    };
    let data = match bincode::serialize(&payload) {
        Ok(d) => d,
        Err(_) => return,
    };
    let _ = std::fs::write(cache_path(hash, game), &data);
}

#[derive(Debug)]
struct PendingCompareEntry {
    str_id: i32,
    source: String,
    record_sig: [u8; 4],
    field_sig: [u8; 4],
    form_id: u32,
    form_owner_hash: u32,
    local_form_id: u32,
    field_index: u16,
    field_index_max: u16,
    edid_hash: u32,
    source_hash: u32,
}

impl PendingCompareEntry {
    fn finish(self, id: u32) -> CompareEntry {
        CompareEntry {
            id,
            str_id: self.str_id,
            source: self.source,
            record_sig: self.record_sig,
            field_sig: self.field_sig,
            form_id: self.form_id,
            form_owner_hash: self.form_owner_hash,
            local_form_id: self.local_form_id,
            field_index: self.field_index,
            field_index_max: self.field_index_max,
            edid_hash: self.edid_hash,
            source_hash: self.source_hash,
        }
    }
}

struct CompareExtractor {
    record_defs: Vec<TranslatableField>,
    def_map: HashMap<([u8; 4], [u8; 4]), usize>,
    strings_files: StringsFiles,
    master_owner_hashes: Vec<u32>,
    self_owner_hash: u32,
    entries: Vec<CompareEntry>,
}

impl CompareExtractor {
    fn new(data_dir: &Path, game: GameId, strings_dir: &Path, base_name: &str) -> Self {
        let record_defs = load_game_record_defs(data_dir, game)
            .unwrap_or_else(|_| parse_record_defs(include_str!("../esp_default_defs.txt")));
        let def_map = build_def_map(&record_defs);
        let strings_files = StringsFiles::load_from_dir(strings_dir, base_name);
        Self {
            record_defs,
            def_map,
            strings_files,
            master_owner_hashes: Vec::new(),
            self_owner_hash: string_hash("\0compare-self\0"),
            entries: Vec::new(),
        }
    }

    fn parse_reader<R: Read>(&mut self, reader: &mut R) -> IoResult<()> {
        loop {
            let header = match GenericHeader::read_from(reader) {
                Ok(header) => header,
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            if header.is_grup() {
                // Delphi ESPCompare 仅读取 GRUP 头部然后继续流式读取；
                // 子记录按文件顺序紧随其后。
                let _ = GrupHeader::read_from(reader)?;
                continue;
            }

            let record_header = RecordHeaderData::read_from(reader)?;
            let mut record_data = vec![0u8; header.dsize as usize];
            reader.read_exact(&mut record_data)?;

            let fields = if record_header.is_compressed() {
                match decompress_bethesda_record(&record_data) {
                    Ok(decompressed) => decompressed,
                    Err(_) => continue,
                }
            } else {
                record_data
            };

            if header.is_tes4() {
                self.capture_tes4_masters(&fields)?;
            }
            self.parse_record_fields(&header.name, record_header.form_id, &fields)?;
        }

        Ok(())
    }

    fn parse_record_fields(
        &mut self,
        record_sig: &[u8; 4],
        form_id: u32,
        data: &[u8],
    ) -> IoResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        let mut cursor = Cursor::new(data);
        let (form_owner_hash, local_form_id) = self.normalize_form_identity(form_id);
        let mut next_field_size: u32 = 0;
        let mut edid: Option<String> = None;
        let mut pending = Vec::new();
        let mut occurrence_by_field: HashMap<[u8; 4], u16> = HashMap::new();
        let mut absolute_field_index = 0u16;

        while cursor.position() < data.len() as u64 {
            let field_header = match FieldHeader::read_from(&mut cursor) {
                Ok(header) => header,
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            let data_size = if next_field_size > 0 {
                let size = next_field_size;
                next_field_size = 0;
                size
            } else {
                field_header.dsize as u32
            };

            let remaining = data.len() as u64 - cursor.position();
            if data_size as u64 > remaining {
                break;
            }

            let mut field_data = vec![0u8; data_size as usize];
            cursor.read_exact(&mut field_data)?;

            if field_header.is_xxxx() {
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

            if &field_header.name == b"EDID" && !field_data.is_empty() {
                let len = field_data
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(field_data.len());
                edid = Some(String::from_utf8_lossy(&field_data[..len]).to_string());
            }

            if let Some(def) = self.find_def(record_sig, &field_header.name) {
                if record_sig == b"GMST" && &field_header.name == b"DATA" {
                    let is_string_gmst = edid.as_ref().map(|e| e.starts_with('s')).unwrap_or(false);
                    if !is_string_gmst {
                        absolute_field_index = absolute_field_index.saturating_add(1);
                        continue;
                    }
                }

                if field_data.len() >= 4 {
                    let string_id = u32::from_le_bytes([
                        field_data[0],
                        field_data[1],
                        field_data[2],
                        field_data[3],
                    ]);
                    let source = self
                        .strings_files
                        .get(def.list_index, string_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<ID:{}>", string_id));

                    if !source.is_empty() {
                        let occurrence = occurrence_by_field.entry(field_header.name).or_insert(0);
                        let field_index = *occurrence;
                        *occurrence = (*occurrence).saturating_add(1);

                        pending.push(PendingCompareEntry {
                            str_id: string_id as i32,
                            source_hash: string_hash(&source),
                            source,
                            record_sig: *record_sig,
                            field_sig: field_header.name,
                            form_id,
                            form_owner_hash,
                            local_form_id,
                            field_index,
                            field_index_max: 0,
                            edid_hash: edid.as_ref().map_or(0, |s| string_hash(s)),
                        });
                    }
                }
            }

            if &field_header.name == b"VMAD" && !field_data.is_empty() {
                self.parse_vmad_entries(
                    record_sig,
                    form_id,
                    form_owner_hash,
                    local_form_id,
                    absolute_field_index,
                    &field_data,
                    &mut pending,
                    &mut occurrence_by_field,
                );
            }

            absolute_field_index = absolute_field_index.saturating_add(1);
        }

        apply_field_index_max(&mut pending, &occurrence_by_field);
        self.entries.reserve(pending.len());
        for entry in pending {
            let id = self.entries.len() as u32;
            self.entries.push(entry.finish(id));
        }

        Ok(())
    }

    fn parse_vmad_entries(
        &self,
        record_sig: &[u8; 4],
        form_id: u32,
        form_owner_hash: u32,
        local_form_id: u32,
        absolute_field_index: u16,
        data: &[u8],
        pending: &mut Vec<PendingCompareEntry>,
        occurrence_by_field: &mut HashMap<[u8; 4], u16>,
    ) {
        if data.len() < 2 {
            return;
        }

        let vmad_version = i16::from_le_bytes([data[0], data[1]]);
        let decoder = VmadDecoder::new(data, vmad_version);

        for vmad_str in decoder.decode() {
            if vmad_str.value.is_empty() {
                continue;
            }

            let occurrence = occurrence_by_field.entry(*b"VMAD").or_insert(0);
            let field_index = *occurrence;
            *occurrence = (*occurrence).saturating_add(1);
            let script_prop_key = format!("{}\0{}", vmad_str.script_name, vmad_str.prop_name);

            pending.push(PendingCompareEntry {
                str_id: -(vmad_str.offset as i32),
                source_hash: string_hash(&vmad_str.value),
                source: vmad_str.value,
                record_sig: *record_sig,
                field_sig: *b"VMAD",
                form_id,
                form_owner_hash,
                local_form_id,
                field_index,
                field_index_max: absolute_field_index,
                edid_hash: string_hash(&script_prop_key),
            });
        }
    }

    fn capture_tes4_masters(&mut self, data: &[u8]) -> IoResult<()> {
        let mut cursor = Cursor::new(data);
        let mut next_field_size: u32 = 0;
        self.master_owner_hashes.clear();

        while cursor.position() < data.len() as u64 {
            let field_header = match FieldHeader::read_from(&mut cursor) {
                Ok(header) => header,
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            let data_size = if next_field_size > 0 {
                let size = next_field_size;
                next_field_size = 0;
                size
            } else {
                field_header.dsize as u32
            };

            let remaining = data.len() as u64 - cursor.position();
            if data_size as u64 > remaining {
                break;
            }

            let mut field_data = vec![0u8; data_size as usize];
            cursor.read_exact(&mut field_data)?;

            if field_header.is_xxxx() {
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

            if &field_header.name == b"MAST" && !field_data.is_empty() {
                let len = field_data
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(field_data.len());
                let master_name = String::from_utf8_lossy(&field_data[..len])
                    .trim()
                    .to_ascii_lowercase();
                self.master_owner_hashes.push(string_hash(&master_name));
            }
        }

        Ok(())
    }

    fn normalize_form_identity(&self, form_id: u32) -> (u32, u32) {
        if form_id == 0 {
            return (0, 0);
        }

        let (owner_index, local_form_id) = split_form_id_identity(form_id);
        let owner_hash = self.owner_hash_for_index(owner_index);

        if owner_hash == 0 {
            (0, form_id)
        } else {
            (owner_hash, local_form_id)
        }
    }

    fn owner_hash_for_index(&self, owner_index: usize) -> u32 {
        if owner_index < self.master_owner_hashes.len() {
            self.master_owner_hashes[owner_index]
        } else if owner_index == self.master_owner_hashes.len() {
            self.self_owner_hash
        } else {
            0
        }
    }

    fn find_def(&self, record_sig: &[u8; 4], field_sig: &[u8; 4]) -> Option<&TranslatableField> {
        let key = (*record_sig, *field_sig);
        if let Some(&idx) = self.def_map.get(&key) {
            return Some(&self.record_defs[idx]);
        }
        let wildcard_key = (*b"****", *field_sig);
        self.def_map
            .get(&wildcard_key)
            .map(|&idx| &self.record_defs[idx])
    }
}

fn build_def_map(defs: &[TranslatableField]) -> HashMap<([u8; 4], [u8; 4]), usize> {
    let mut map = HashMap::new();
    for (i, def) in defs.iter().enumerate() {
        if !def.ignored {
            map.insert((def.record_sig, def.field_sig), i);
        }
    }
    map
}

fn apply_field_index_max(
    entries: &mut [PendingCompareEntry],
    occurrence_by_field: &HashMap<[u8; 4], u16>,
) {
    for entry in entries {
        let max_index = occurrence_by_field
            .get(&entry.field_sig)
            .copied()
            .unwrap_or(0)
            .saturating_sub(1);
        if max_index > 0 {
            entry.field_index_max = max_index;
        }
    }
}

fn extract_compare_entries(
    esp_path: &Path,
    data_dir: &Path,
    game: GameId,
) -> Result<Vec<CompareEntry>, String> {
    let base_name = esp_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let esp_dir = esp_path.parent().unwrap_or_else(|| Path::new("."));

    let mut extractor = CompareExtractor::new(data_dir, game, esp_dir, base_name);
    let file = std::fs::File::open(esp_path).map_err(|e| format!("Failed to open ESP: {}", e))?;
    extractor
        .parse_reader(&mut BufReader::new(file))
        .map_err(|e| format!("Failed to parse ESP: {}", e))?;

    Ok(extractor.entries)
}

/// 解析 ESP 并加载字符串，使用轻量级对比缓存
fn parse_esp_with_entries(
    esp_path: &str,
    data_dir: &Path,
    game: GameId,
) -> Result<Vec<CompareEntry>, String> {
    let esp_path_ref = Path::new(esp_path);
    let hash = file_hash(esp_path_ref);

    // 先尝试从轻量对比缓存加载（~200ms vs ~2.5s for full cache）
    if let Some(hash) = hash.as_deref() {
        if let Some(cached) = load_cached_entries(hash, game) {
            return Ok(cached);
        }
    }

    // 缓存未命中，按原版 ESPCompare 思路只抽取可比较字符串字段。
    let entries = extract_compare_entries(esp_path_ref, data_dir, game)?;
    if let Some(hash) = hash.as_deref() {
        store_cached_entries(hash, game, &entries);
    }

    Ok(entries)
}

/// 对比两个 ESP 文件
///
/// 返回包含已匹配和未匹配字符串 ID 的 EspComparison。
/// 从每个 ESP 的父目录加载 Strings 文件，以确保准确的源文本显示。
pub fn compare_esp_files(
    old_esp_path: &str,
    new_esp_path: &str,
    data_dir: Option<&str>,
    game: GameId,
) -> Result<EspComparison, String> {
    let data_path = Path::new(data_dir.unwrap_or("Data"));

    // 短路：自比较只需解析一次
    let same_file = paths_same(old_esp_path, new_esp_path);
    let old_entries = parse_esp_with_entries(old_esp_path, data_path, game)
        .map_err(|e| format!("Failed to parse old ESP: {}", e))?;
    let new_entries = if same_file {
        old_entries.clone()
    } else {
        parse_esp_with_entries(new_esp_path, data_path, game)
            .map_err(|e| format!("Failed to parse new ESP: {}", e))?
    };

    Ok(build_comparison_from_entries(old_entries, new_entries))
}

/// 检查两个路径是否指向同一个文件（规范化路径或回退到字符串相等性比较）
fn paths_same(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(pa), Ok(pb)) => pa == pb,
        _ => false,
    }
}

/// 对比两组 SkyString（已解析）
///
/// 适用于将 ESP 与加载的 SST/XML 词典进行对比，或用于单元测试。
pub fn compare_string_sets(old_strings: &[SkyString], new_strings: &[SkyString]) -> EspComparison {
    let old_entries: Vec<CompareEntry> = old_strings
        .iter()
        .map(CompareEntry::from_sky_string)
        .collect();
    let new_entries: Vec<CompareEntry> = new_strings
        .iter()
        .map(CompareEntry::from_sky_string)
        .collect();
    build_comparison_from_entries(old_entries, new_entries)
}

/// 从两个 CompareEntry 向量构建对比结果（轻量级）
fn build_comparison_from_entries(
    old_entries: Vec<CompareEntry>,
    new_entries: Vec<CompareEntry>,
) -> EspComparison {
    // 构建键→旧索引映射。保留索引避免克隆每个 CompareEntry 到查找映射中，
    // 这对完整主文件很重要。
    let mut old_by_key: HashMap<StringKey, usize> = HashMap::with_capacity(old_entries.len());
    for (index, e) in old_entries.iter().enumerate() {
        let key = StringKey::from_compare_entry(e);
        old_by_key.insert(key, index);
    }

    // 使用 HashSet 进行 O(1) 的 is_added/is_removed/is_modified 查找
    let mut matched_pairs = HashMap::with_capacity(new_entries.len().min(old_entries.len()));
    let mut added = HashSet::with_capacity(new_entries.len());
    let mut removed = HashSet::with_capacity(old_entries.len());
    let mut modified = HashSet::with_capacity(old_entries.len() / 10); // estimate ~10% modified
    let mut modified_pairs = HashMap::with_capacity(old_entries.len() / 10);
    let mut matched_old_indexes = vec![false; old_entries.len()];
    let mut seen_new_keys = HashSet::with_capacity(new_entries.len());

    // 反向迭代以保留之前 HashMap "后键覆盖" 的行为，
    // 避免为重复字符串键构建第二个键→新索引映射。
    for new_e in new_entries.iter().rev() {
        let key = StringKey::from_compare_entry(new_e);
        if !seen_new_keys.insert(key) {
            continue;
        }

        if let Some(&old_index) = old_by_key.get(&key) {
            matched_old_indexes[old_index] = true;
            let old_e = &old_entries[old_index];
            if old_e.source_hash == new_e.source_hash && old_e.source == new_e.source {
                // 精确匹配
                matched_pairs.insert(new_e.id, old_e.id);
            } else {
                // 已修改（相同键，不同文本）
                modified.insert(new_e.id);
                modified_pairs.insert(new_e.id, old_e.id);
            }
        } else {
            added.insert(new_e.id);
        }
    }

    // 已删除条目：旧键在新数据中未找到
    for &old_index in old_by_key.values() {
        if !matched_old_indexes[old_index] {
            removed.insert(old_entries[old_index].id);
        }
    }

    EspComparison {
        old_strings: old_entries,
        new_strings: new_entries,
        matched_pairs,
        added,
        removed,
        modified,
        modified_pairs,
    }
}

impl EspComparison {
    /// 获取完全相同字符串的数量
    pub fn identical_count(&self) -> usize {
        self.matched_pairs.len()
    }

    /// 获取新增字符串的数量
    pub fn added_count(&self) -> usize {
        self.added.len()
    }

    /// 获取已删除字符串的数量
    pub fn removed_count(&self) -> usize {
        self.removed.len()
    }

    /// 获取已修改字符串的数量
    pub fn modified_count(&self) -> usize {
        self.modified.len()
    }

    /// 获取对比中所有唯一的字符串 ID
    pub fn all_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.new_strings.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// 获取给定新字符串 ID 的匹配对 (new_id, old_id)
    pub fn get_match(&self, new_id: u32) -> Option<u32> {
        self.matched_pairs.get(&new_id).copied()
    }

    /// 检查字符串 ID 是否为新增 (added) — 通过 HashSet 达到 O(1) 复杂度
    pub fn is_added(&self, new_id: u32) -> bool {
        self.added.contains(&new_id)
    }

    /// 检查字符串 ID 是否已被删除 — 通过 HashSet 达到 O(1) 复杂度
    pub fn is_removed(&self, old_id: u32) -> bool {
        self.removed.contains(&old_id)
    }

    /// 检查字符串 ID 是否已被修改 — 通过 HashSet 达到 O(1) 复杂度
    pub fn is_modified(&self, new_id: u32) -> bool {
        self.modified.contains(&new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::esp_pointer::EspPointer;
    use crate::types::sky_string::SkyString;

    fn push_field(data: &mut Vec<u8>, sig: &[u8; 4], value: u32) {
        data.extend_from_slice(sig);
        data.extend_from_slice(&4u16.to_le_bytes());
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn make_test_string(id: u32, str_id: i32, source: &str, record_sig: &[u8; 4]) -> SkyString {
        make_test_string_with_pointer(id, str_id, source, record_sig, 0, 0)
    }

    fn make_test_string_with_pointer(
        id: u32,
        str_id: i32,
        source: &str,
        record_sig: &[u8; 4],
        form_id: u32,
        field_index: u16,
    ) -> SkyString {
        SkyString {
            id,
            source: source.to_string(),
            translation: String::new(),
            record_sig: *record_sig,
            field_sig: *b"TEST",
            esp_ptr: EspPointer {
                str_id,
                form_id,
                record_sig: *record_sig,
                field_sig: *b"TEST",
                index: field_index,
                index_max: 0,
                edid_hash: 0,
            },
            params: Default::default(),
            internal_params: Default::default(),
            list_index: 0,
            colab_id: 0,
            ld_result: 0.0,
            ld_found: 0,
            min_word: 0,
            source_normalized: None,
            normalized_hash: None,
            edid: None,
            hash: 0,
            hash_trans: 0,
            word_hashes: Vec::new(),
            rec_refs: Vec::new(),
            parent_form_id: 0,
            tag_hash: 0,
            field_ref: None,
        }
    }

    fn make_compare_entry(
        id: u32,
        source: &str,
        form_id: u32,
        form_owner_hash: u32,
        local_form_id: u32,
        field_index: u16,
    ) -> CompareEntry {
        CompareEntry {
            id,
            str_id: 0,
            source: source.to_string(),
            record_sig: *b"TEST",
            field_sig: *b"FULL",
            form_id,
            form_owner_hash,
            local_form_id,
            field_index,
            field_index_max: 0,
            edid_hash: 0,
            source_hash: string_hash(source),
        }
    }

    #[test]
    fn test_compare_identical() {
        let old = vec![
            make_test_string(0, 1, "Hello", b"TEST"),
            make_test_string(1, 2, "World", b"TEST"),
        ];
        let new = vec![
            make_test_string(10, 1, "Hello", b"TEST"),
            make_test_string(11, 2, "World", b"TEST"),
        ];

        let comp = compare_string_sets(&old, &new);

        assert_eq!(comp.identical_count(), 2);
        assert_eq!(comp.added_count(), 0);
        assert_eq!(comp.removed_count(), 0);
        assert_eq!(comp.modified_count(), 0);
    }

    #[test]
    fn test_compare_added_removed() {
        let old = vec![make_test_string(0, 1, "Hello", b"TEST")];
        let new = vec![
            make_test_string(10, 1, "Hello", b"TEST"),
            make_test_string(11, 2, "New", b"TEST"),
        ];

        let comp = compare_string_sets(&old, &new);

        assert_eq!(comp.identical_count(), 1);
        assert_eq!(comp.added_count(), 1);
        assert_eq!(comp.removed_count(), 0);
    }

    #[test]
    fn test_compare_modified() {
        let old = vec![make_test_string(0, 1, "Hello", b"TEST")];
        let new = vec![make_test_string(10, 1, "Hola", b"TEST")];

        let comp = compare_string_sets(&old, &new);

        assert_eq!(comp.identical_count(), 0);
        assert_eq!(comp.modified_count(), 1);
        assert_eq!(comp.added_count(), 0);
        assert_eq!(comp.removed_count(), 0);
    }

    #[test]
    fn test_compare_prefers_form_id_over_shared_string_id() {
        let old = vec![make_test_string_with_pointer(
            0,
            1,
            "Hello",
            b"TEST",
            0x0100_0001,
            0,
        )];
        let new = vec![make_test_string_with_pointer(
            10,
            1,
            "Hola",
            b"TEST",
            0x0100_0002,
            0,
        )];

        let comp = compare_string_sets(&old, &new);

        assert_eq!(comp.identical_count(), 0);
        assert_eq!(comp.modified_count(), 0);
        assert_eq!(comp.added_count(), 1);
        assert_eq!(comp.removed_count(), 1);
    }

    #[test]
    fn test_compare_uses_duplicate_field_occurrence() {
        let old = vec![
            make_test_string_with_pointer(0, 1, "First", b"TEST", 0x0100_0001, 0),
            make_test_string_with_pointer(1, 2, "Second", b"TEST", 0x0100_0001, 1),
        ];
        let new = vec![
            make_test_string_with_pointer(10, 1, "First", b"TEST", 0x0100_0001, 0),
            make_test_string_with_pointer(11, 2, "Second changed", b"TEST", 0x0100_0001, 1),
        ];

        let comp = compare_string_sets(&old, &new);

        assert_eq!(comp.identical_count(), 1);
        assert_eq!(comp.modified_count(), 1);
        assert_eq!(comp.added_count(), 0);
        assert_eq!(comp.removed_count(), 0);
    }

    #[test]
    fn test_compare_extractor_keeps_only_string_fields_with_occurrence_indexes() {
        let record_defs = vec![TranslatableField::new(*b"TEST", *b"FULL", 0)];
        let def_map = build_def_map(&record_defs);
        let mut extractor = CompareExtractor {
            record_defs,
            def_map,
            strings_files: StringsFiles::default(),
            master_owner_hashes: Vec::new(),
            self_owner_hash: string_hash("\0compare-self\0"),
            entries: Vec::new(),
        };
        let mut data = Vec::new();
        push_field(&mut data, b"FULL", 1);
        push_field(&mut data, b"EDID", 99);
        push_field(&mut data, b"FULL", 2);

        extractor
            .parse_record_fields(b"TEST", 0x0100_0001, &data)
            .unwrap();

        assert_eq!(extractor.entries.len(), 2);
        assert_eq!(extractor.entries[0].source, "<ID:1>");
        assert_eq!(extractor.entries[0].field_index, 0);
        assert_eq!(extractor.entries[0].field_index_max, 1);
        assert_eq!(extractor.entries[1].source, "<ID:2>");
        assert_eq!(extractor.entries[1].field_index, 1);
        assert_eq!(extractor.entries[1].field_index_max, 1);
    }

    #[test]
    fn test_compare_uses_normalized_master_identity() {
        let owner_hash = string_hash("skyrim.esm");
        let old = vec![make_compare_entry(
            0,
            "Shared",
            0x0100_0042,
            owner_hash,
            0x0000_0042,
            0,
        )];
        let new = vec![make_compare_entry(
            10,
            "Shared",
            0x0200_0042,
            owner_hash,
            0x0000_0042,
            0,
        )];

        let comp = build_comparison_from_entries(old, new);

        assert_eq!(comp.identical_count(), 1);
        assert_eq!(comp.added_count(), 0);
        assert_eq!(comp.removed_count(), 0);
    }

    #[test]
    fn test_compare_large_dataset_performance() {
        // 性能测试：75K 条字符串对比
        let count = 75_000;
        let old: Vec<SkyString> = (0..count)
            .map(|i| make_test_string(i, i as i32, &format!("Source text {}", i), b"TEST"))
            .collect();
        let new: Vec<SkyString> = (0..count)
            .map(|i| make_test_string(i + count, i as i32, &format!("Source text {}", i), b"TEST"))
            .collect();

        let start = std::time::Instant::now();
        let comp = compare_string_sets(&old, &new);
        let elapsed = start.elapsed();

        assert_eq!(comp.identical_count(), count as usize);
        assert_eq!(comp.added_count(), 0);
        assert_eq!(comp.removed_count(), 0);

        // 验证查询性能（使用 HashSet 应当达到 O(1) 复杂度）
        let query_start = std::time::Instant::now();
        for i in 0..count as u32 {
            let _ = comp.is_added(i);
            let _ = comp.is_removed(i);
            let _ = comp.is_modified(i);
        }
        let query_elapsed = query_start.elapsed();

        // 对比和查询应该在合理时间内完成
        assert!(
            elapsed.as_millis() < 5000,
            "Comparison took {}ms",
            elapsed.as_millis()
        );
        assert!(
            query_elapsed.as_millis() < 1000,
            "Queries took {}ms",
            query_elapsed.as_millis()
        );
    }
}
