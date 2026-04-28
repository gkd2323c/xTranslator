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
pub fn compare_esp_files(
    old_esp_path: &str,
    new_esp_path: &str,
    data_dir: Option<&str>,
    game: GameId,
) -> Result<EspComparison, String> {
    let data_path = Path::new(data_dir.unwrap_or("Data"));

    // Parse old ESP
    let mut old_parser = EspParser::with_game(data_path, game)
        .map_err(|e| format!("Failed to create parser for old ESP: {}", e))?;
    let old_file = std::fs::File::open(old_esp_path)
        .map_err(|e| format!("Failed to open old ESP: {}", e))?;
    old_parser.parse(&mut std::io::BufReader::new(old_file))
        .map_err(|e| format!("Failed to parse old ESP: {}", e))?;

    // Parse new ESP
    let mut new_parser = EspParser::with_game(data_path, game)
        .map_err(|e| format!("Failed to create parser for new ESP: {}", e))?;
    let new_file = std::fs::File::open(new_esp_path)
        .map_err(|e| format!("Failed to open new ESP: {}", e))?;
    new_parser.parse(&mut std::io::BufReader::new(new_file))
        .map_err(|e| format!("Failed to parse new ESP: {}", e))?;

    Ok(build_comparison(old_parser.strings, new_parser.strings))
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
    // Build key -> (internal_id, SkyString) maps
    let mut old_by_key: HashMap<StringKey, (u32, SkyString)> = HashMap::new();
    for s in &old_strings {
        let key = StringKey::from_sky_string(s);
        old_by_key.insert(key, (s.id, s.clone()));
    }

    let mut new_by_key: HashMap<StringKey, (u32, SkyString)> = HashMap::new();
    for s in &new_strings {
        let key = StringKey::from_sky_string(s);
        new_by_key.insert(key, (s.id, s.clone()));
    }

    let mut matched_pairs = HashMap::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut modified_pairs = HashMap::new();

    // Check old entries: find matches, removals, modifications
    for (key, (old_id, old_s)) in &old_by_key {
        if let Some((new_id, new_s)) = new_by_key.get(key) {
            if old_s.source == new_s.source {
                // Exact match
                matched_pairs.insert(*new_id, *old_id);
            } else {
                // Modified (same key, different text)
                modified.push(*new_id);
                modified_pairs.insert(*new_id, *old_id);
            }
        } else {
            // Removed from old
            removed.push(*old_id);
        }
    }

    // Find added entries (in new but not in old)
    let mut added = Vec::new();
    for (key, (new_id, _)) in &new_by_key {
        if !old_by_key.contains_key(key) {
            added.push(*new_id);
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
