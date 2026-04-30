//! ESP file comparison — build string pairs between two plugin files
//!
//! Compares two ESP/ESM files and produces a mapping of string pairs:
//! - identical: strings match exactly
//! - modified: same str_id but different text
//! - added: present in new but not in old
//! - removed: present in old but not in new
//!
//! The comparison is based on (str_id, record_sig, field_sig) triple, which
//! matches the Delphi xTranslator XML import matching strategy.
//!
//! Uses a lightweight compare-specific cache (avoids storing full SkyString
//! with normalization/hashes/etc.) for fast deserialization.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::esp::parser::EspParser;
use crate::types::game_id::GameId;
use crate::types::sky_string::SkyString;

/// Lightweight entry for comparison — stores only the fields needed
/// for StringKey matching and source comparison. Much smaller and faster
/// to deserialize than full SkyString (~200ms vs ~1500ms for 75K entries).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompareEntry {
    pub id: u32,
    pub str_id: i32,
    pub source: String,
    pub record_sig: [u8; 4],
    pub field_sig: [u8; 4],
    pub form_id: u32,
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
        }
    }
}

/// Comparison result between two ESP files
#[derive(Debug, Clone)]
pub struct EspComparison {
    /// All strings from old ESP (by internal ID)
    pub old_strings: Vec<CompareEntry>,
    /// All strings from new ESP (by internal ID)
    pub new_strings: Vec<CompareEntry>,
    /// Mapping: new internal ID -> old internal ID for matching entries
    pub matched_pairs: HashMap<u32, u32>,
    /// Strings in new but not in old (HashSet for O(1) lookup)
    pub added: HashSet<u32>,
    /// Strings in old but not in new (HashSet for O(1) lookup)
    pub removed: HashSet<u32>,
    /// Strings with same key but different text (HashSet for O(1) lookup)
    pub modified: HashSet<u32>,
    /// Mapping: new ID -> old ID for modified entries (same key, different text)
    pub modified_pairs: HashMap<u32, u32>,
}

/// Key used for matching strings between ESP files
///
/// Matches Delphi xTranslator's (str_id, record_sig, field_sig) triple.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct StringKey {
    pub str_id: i32,
    pub record_sig: [u8; 4],
    pub field_sig: [u8; 4],
}

impl StringKey {
    fn from_compare_entry(e: &CompareEntry) -> Self {
        Self {
            str_id: e.str_id,
            record_sig: e.record_sig,
            field_sig: e.field_sig,
        }
    }
}

const COMPARE_CACHE_VERSION: u32 = 1;

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

fn cache_path(hash: &str) -> PathBuf {
    compare_cache_dir().join(format!("{}.compare", hash))
}

/// Load cached CompareEntries from disk (if available and valid)
fn load_cached_entries(hash: &str) -> Option<Vec<CompareEntry>> {
    let path = cache_path(hash);
    if !path.exists() {
        return None;
    }
    let data = std::fs::read(&path).ok()?;
    let (version, entries): (u32, Vec<CompareEntry>) = bincode::deserialize(&data).ok()?;
    if version != COMPARE_CACHE_VERSION {
        let _ = std::fs::remove_file(&path);
        return None;
    }
    Some(entries)
}

/// Store CompareEntries to disk for future fast loading
fn store_cached_entries(hash: &str, entries: &[CompareEntry]) {
    let dir = compare_cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    let data = match bincode::serialize(&(COMPARE_CACHE_VERSION, entries)) {
        Ok(d) => d,
        Err(_) => return,
    };
    let _ = std::fs::write(cache_path(hash), &data);
}

/// Parse ESP and load strings, using lightweight compare cache
fn parse_esp_with_entries(
    esp_path: &str,
    data_dir: &Path,
    game: GameId,
) -> Result<Vec<CompareEntry>, String> {
    let esp_path_ref = Path::new(esp_path);
    let hash = file_hash(esp_path_ref);

    // 先尝试从轻量对比缓存加载（~200ms vs ~2.5s for full cache）
    if let Some(hash) = hash.as_deref() {
        if let Some(cached) = load_cached_entries(hash) {
            return Ok(cached);
        }
    }

    // 缓存未命中，完整解析
    let mut parser = EspParser::with_game(data_dir, game)
        .map_err(|e| format!("Failed to create parser: {}", e))?;
    parser.set_build_search_index(false);

    let base_name = esp_path_ref
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let esp_dir = esp_path_ref.parent().unwrap_or_else(|| Path::new("."));
    parser.load_strings_files(esp_dir, base_name);

    let file = std::fs::File::open(esp_path).map_err(|e| format!("Failed to open ESP: {}", e))?;
    parser
        .parse(&mut std::io::BufReader::new(file))
        .map_err(|e| format!("Failed to parse ESP: {}", e))?;

    // Store lightweight compare entries to cache
    let entries: Vec<CompareEntry> = parser
        .strings
        .iter()
        .map(CompareEntry::from_sky_string)
        .collect();
    if let Some(hash) = hash.as_deref() {
        store_cached_entries(hash, &entries);
    }

    Ok(entries)
}

/// Compare two ESP files
///
/// Returns an EspComparison with matched and unmatched string IDs.
/// Strings files are loaded from each ESP's parent directory for accurate source display.
pub fn compare_esp_files(
    old_esp_path: &str,
    new_esp_path: &str,
    data_dir: Option<&str>,
    game: GameId,
) -> Result<EspComparison, String> {
    let data_path = Path::new(data_dir.unwrap_or("Data"));

    // Short-circuit: self-compare only needs one parse
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

/// Check if two paths refer to the same file (canonicalize or fallback to string eq)
fn paths_same(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(pa), Ok(pb)) => pa == pb,
        _ => false,
    }
}

/// Compare two sets of SkyStrings (already parsed)
///
/// Useful for comparing an ESP with a loaded SST/XML dictionary, or
/// for unit testing.
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

/// Build comparison from two CompareEntry vectors (lightweight)
fn build_comparison_from_entries(
    old_entries: Vec<CompareEntry>,
    new_entries: Vec<CompareEntry>,
) -> EspComparison {
    // Build a key -> old index map. Keeping indexes avoids cloning every
    // CompareEntry into lookup maps, which matters for full master files.
    let mut old_by_key: HashMap<StringKey, usize> = HashMap::with_capacity(old_entries.len());
    for (index, e) in old_entries.iter().enumerate() {
        let key = StringKey::from_compare_entry(e);
        old_by_key.insert(key, index);
    }

    // Use HashSet for O(1) lookups in is_added/is_removed/is_modified
    let mut matched_pairs = HashMap::with_capacity(new_entries.len().min(old_entries.len()));
    let mut added = HashSet::with_capacity(new_entries.len());
    let mut removed = HashSet::with_capacity(old_entries.len());
    let mut modified = HashSet::with_capacity(old_entries.len() / 10); // estimate ~10% modified
    let mut modified_pairs = HashMap::with_capacity(old_entries.len() / 10);
    let mut matched_old_indexes = vec![false; old_entries.len()];
    let mut seen_new_keys = HashSet::with_capacity(new_entries.len());

    // Iterate in reverse to preserve the previous HashMap "last key wins" behavior
    // for duplicate string keys, without building a second key -> new index map.
    for new_e in new_entries.iter().rev() {
        let key = StringKey::from_compare_entry(new_e);
        if !seen_new_keys.insert(key) {
            continue;
        }

        if let Some(&old_index) = old_by_key.get(&key) {
            matched_old_indexes[old_index] = true;
            let old_e = &old_entries[old_index];
            if old_e.source == new_e.source {
                // Exact match
                matched_pairs.insert(new_e.id, old_e.id);
            } else {
                // Modified (same key, different text)
                modified.insert(new_e.id);
                modified_pairs.insert(new_e.id, old_e.id);
            }
        } else {
            added.insert(new_e.id);
        }
    }

    // Removed entries: old keys not found in new
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
    /// Get the count of identical strings
    pub fn identical_count(&self) -> usize {
        self.matched_pairs.len()
    }

    /// Get the count of added strings
    pub fn added_count(&self) -> usize {
        self.added.len()
    }

    /// Get the count of removed strings
    pub fn removed_count(&self) -> usize {
        self.removed.len()
    }

    /// Get the count of modified strings
    pub fn modified_count(&self) -> usize {
        self.modified.len()
    }

    /// Get all unique string IDs in the comparison
    pub fn all_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.new_strings.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// Get matching pair (new_id, old_id) for a given new string ID
    pub fn get_match(&self, new_id: u32) -> Option<u32> {
        self.matched_pairs.get(&new_id).copied()
    }

    /// Check if a string ID is new (added) — O(1) via HashSet
    pub fn is_added(&self, new_id: u32) -> bool {
        self.added.contains(&new_id)
    }

    /// Check if a string ID was removed — O(1) via HashSet
    pub fn is_removed(&self, old_id: u32) -> bool {
        self.removed.contains(&old_id)
    }

    /// Check if a string ID was modified — O(1) via HashSet
    pub fn is_modified(&self, new_id: u32) -> bool {
        self.modified.contains(&new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::esp_pointer::EspPointer;
    use crate::types::sky_string::SkyString;

    fn make_test_string(id: u32, str_id: i32, source: &str, record_sig: &[u8; 4]) -> SkyString {
        SkyString {
            id,
            source: source.to_string(),
            translation: String::new(),
            record_sig: *record_sig,
            field_sig: *b"TEST",
            esp_ptr: EspPointer {
                str_id,
                form_id: 0,
                record_sig: *record_sig,
                field_sig: *b"TEST",
                index: 0,
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
            hash: 0,
            hash_trans: 0,
            word_hashes: Vec::new(),
            rec_refs: Vec::new(),
            parent_form_id: 0,
            tag_hash: 0,
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

        // 验证查询性能 (should be O(1) with HashSet)
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
