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

use std::collections::HashMap;
use std::path::Path;

use crate::esp::parser::EspParser;
use crate::types::game_id::GameId;
use crate::types::sky_string::SkyString;

/// Comparison result between two ESP files
#[derive(Debug, Clone)]
pub struct EspComparison {
    /// All strings from old ESP (by internal ID)
    pub old_strings: Vec<SkyString>,
    /// All strings from new ESP (by internal ID)
    pub new_strings: Vec<SkyString>,
    /// Mapping: new internal ID -> old internal ID for matching entries
    pub matched_pairs: HashMap<u32, u32>,
    /// Strings in new but not in old
    pub added: Vec<u32>,
    /// Strings in old but not in new
    pub removed: Vec<u32>,
    /// Strings with same key but different text
    pub modified: Vec<u32>,
    /// Mapping: new ID -> old ID for modified entries (same key, different text)
    pub modified_pairs: HashMap<u32, u32>,
}

/// Key used for matching strings between ESP files
///
/// Matches Delphi xTranslator's (str_id, record_sig, field_sig) triple.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct StringKey {
    pub str_id: i32,
    pub record_sig: [u8; 4],
    pub field_sig: [u8; 4],
}

impl StringKey {
    fn from_sky_string(s: &SkyString) -> Self {
        Self {
            str_id: s.esp_ptr.str_id,
            record_sig: s.esp_ptr.record_sig,
            field_sig: s.esp_ptr.field_sig,
        }
    }
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

    let old_strings = parse_esp_with_strings(old_esp_path, data_path, game)
        .map_err(|e| format!("Failed to parse old ESP: {}", e))?;
    let new_strings = parse_esp_with_strings(new_esp_path, data_path, game)
        .map_err(|e| format!("Failed to parse new ESP: {}", e))?;

    Ok(build_comparison(old_strings, new_strings))
}

fn parse_esp_with_strings(
    esp_path: &str,
    data_dir: &Path,
    game: GameId,
) -> Result<Vec<SkyString>, String> {
    use crate::esp::parser::StringsFiles;
    let mut parser = EspParser::with_game(data_dir, game)
        .map_err(|e| format!("Failed to create parser: {}", e))?;

    let base_name = Path::new(esp_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let esp_dir = Path::new(esp_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    parser.load_strings_files(esp_dir, base_name);

    let file = std::fs::File::open(esp_path)
        .map_err(|e| format!("Failed to open ESP: {}", e))?;
    parser.parse(&mut std::io::BufReader::new(file))
        .map_err(|e| format!("Failed to parse ESP: {}", e))?;

    Ok(parser.strings)
}

/// Compare two sets of SkyStrings (already parsed)
///
/// Useful for comparing an ESP with a loaded SST/XML dictionary, or
/// for unit testing.
pub fn compare_string_sets(
    old_strings: &[SkyString],
    new_strings: &[SkyString],
) -> EspComparison {
    build_comparison(old_strings.to_vec(), new_strings.to_vec())
}

/// Build comparison from two string vectors
fn build_comparison(
    old_strings: Vec<SkyString>,
    new_strings: Vec<SkyString>,
) -> EspComparison {
    // Build key -> index maps. Keeping indexes avoids cloning every SkyString
    // into the lookup maps, which matters for full master files.
    let mut old_by_key: HashMap<StringKey, usize> = HashMap::with_capacity(old_strings.len());
    for (index, s) in old_strings.iter().enumerate() {
        let key = StringKey::from_sky_string(s);
        old_by_key.insert(key, index);
    }

    let mut new_by_key: HashMap<StringKey, usize> = HashMap::with_capacity(new_strings.len());
    for (index, s) in new_strings.iter().enumerate() {
        let key = StringKey::from_sky_string(s);
        new_by_key.insert(key, index);
    }

    let mut matched_pairs = HashMap::with_capacity(new_strings.len().min(old_strings.len()));
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut modified_pairs = HashMap::new();

    // Check old entries: find matches, removals, modifications
    for (key, old_index) in &old_by_key {
        let old_s = &old_strings[*old_index];
        if let Some(new_index) = new_by_key.get(key) {
            let new_s = &new_strings[*new_index];
            if old_s.source == new_s.source {
                // Exact match
                matched_pairs.insert(new_s.id, old_s.id);
            } else {
                // Modified (same key, different text)
                modified.push(new_s.id);
                modified_pairs.insert(new_s.id, old_s.id);
            }
        } else {
            // Removed from old
            removed.push(old_s.id);
        }
    }

    // Find added entries (in new but not in old)
    let mut added = Vec::new();
    for (key, new_index) in &new_by_key {
        if !old_by_key.contains_key(key) {
            added.push(new_strings[*new_index].id);
        }
    }

    EspComparison {
        old_strings,
        new_strings,
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

    /// Check if a string ID is new (added)
    pub fn is_added(&self, new_id: u32) -> bool {
        self.added.contains(&new_id)
    }

    /// Check if a string ID was removed
    pub fn is_removed(&self, old_id: u32) -> bool {
        self.removed.contains(&old_id)
    }

    /// Check if a string ID was modified
    pub fn is_modified(&self, new_id: u32) -> bool {
        self.modified.contains(&new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::sky_string::SkyString;
    use crate::types::esp_pointer::EspPointer;

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
        let old = vec![
            make_test_string(0, 1, "Hello", b"TEST"),
        ];
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
        let old = vec![
            make_test_string(0, 1, "Hello", b"TEST"),
        ];
        let new = vec![
            make_test_string(10, 1, "Hola", b"TEST"),
        ];

        let comp = compare_string_sets(&old, &new);

        assert_eq!(comp.identical_count(), 0);
        assert_eq!(comp.modified_count(), 1);
        assert_eq!(comp.added_count(), 0);
        assert_eq!(comp.removed_count(), 0);
    }
}
