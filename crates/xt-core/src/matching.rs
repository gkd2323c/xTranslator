//! Shared dictionary apply matcher.
//!
//! XML imports and SST loads share the same tiered matching order:
//!
//! | Tier | Strategy | Key | Confidence |
//! |------|----------|-----|------------|
//! | T1 | Exact triple | (str_id, record_sig, field_sig) | very high |
//! | T2 | EDID hash | (edid_hash, record_sig, field_sig) | high |
//! | T3 | Normalized source | (normalized_hash, record_sig, field_sig) | high |
//! | T4 | Vocabulary overlap | word_hashes Jaccard >= 0.5 | medium |
//!
//! Ambiguous matches are not auto-applied.

use std::collections::HashSet;

use crate::normalization;
use crate::types::esp_pointer::{string_hash, HeaderSig};
use crate::types::params::SkyStringParams;
use crate::types::sky_string::SkyString;
use crate::xml::XmlStringEntry;

/// 词汇重叠匹配的最小 Jaccard 阈值
///
/// 阈值过低 → 误匹配风险增加
/// 阈值过高 → 同义改写无法匹配
/// 0.5 意味着至少一半的规范化词汇需要重叠
const MIN_JACCARD: f64 = 0.5;

/// Source format for dictionary entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionarySourceFormat {
    Xml,
    Sst,
}

/// Neutral dictionary entry consumed by the shared matcher.
#[derive(Debug, Clone)]
pub struct DictionaryApplyEntry {
    pub source_format: DictionarySourceFormat,
    pub list_index: u8,
    pub str_id: i32,
    pub record_sig: HeaderSig,
    pub field_sig: HeaderSig,
    pub index: u16,
    pub index_max: u16,
    pub source: String,
    pub translation: String,
    pub edid: Option<String>,
    pub edid_hash: Option<u32>,
    pub params: Option<SkyStringParams>,
}

impl DictionaryApplyEntry {
    pub fn from_xml_entry(entry: &XmlStringEntry) -> Self {
        Self {
            source_format: DictionarySourceFormat::Xml,
            list_index: entry.list_index,
            str_id: entry.str_id,
            record_sig: entry.record_sig,
            field_sig: entry.field_sig,
            index: entry.index,
            index_max: entry.index_max,
            source: entry.source.clone(),
            translation: entry.translation.clone(),
            edid: entry.edid.clone(),
            edid_hash: entry.edid.as_ref().map(|edid| string_hash(edid)),
            params: None,
        }
    }

    pub fn from_sst_entry(entry: &SkyString) -> Self {
        Self {
            source_format: DictionarySourceFormat::Sst,
            list_index: entry.list_index,
            str_id: entry.esp_ptr.str_id,
            record_sig: entry.esp_ptr.record_sig,
            field_sig: entry.esp_ptr.field_sig,
            index: entry.esp_ptr.index,
            index_max: entry.esp_ptr.index_max,
            source: entry.source.clone(),
            translation: entry.translation.clone(),
            edid: None,
            edid_hash: if entry.esp_ptr.edid_hash == 0 {
                None
            } else {
                Some(entry.esp_ptr.edid_hash)
            },
            params: Some(entry.params),
        }
    }
}

/// Matching tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    /// Exact triple match.
    Exact,
    /// EDID hash match.
    Edid,
    /// Normalized source match.
    Normalized,
    /// Vocabulary overlap match.
    Vocab,
}

/// Shared dictionary application result.
#[derive(Debug, Clone, Default)]
pub struct MatchResult {
    /// Tier 1 exact match count.
    pub tier_exact: u32,
    /// Tier 2 EDID match count.
    pub tier_edid: u32,
    /// Tier 3 normalized source match count.
    pub tier_normalized: u32,
    /// Tier 4 vocabulary match count.
    pub tier_vocab: u32,
    /// Ambiguous entries that were not auto-applied.
    pub ambiguous: u32,
    /// Unmatched entries.
    pub unmatched: u32,
    /// Updated SkyString IDs.
    pub updated_ids: Vec<u32>,
}

impl MatchResult {
    /// Total applied matches.
    pub fn total_matched(&self) -> u32 {
        self.tier_exact + self.tier_edid + self.tier_normalized + self.tier_vocab
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TierMatch {
    None,
    Unique(usize),
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryOutcome {
    Applied(MatchTier, usize),
    Ambiguous,
    Unmatched,
}

/// Apply dictionary entries through the shared matcher.
pub fn apply_dictionary_entries(
    strings: &mut [SkyString],
    entries: &[DictionaryApplyEntry],
) -> MatchResult {
    let mut result = MatchResult::default();
    let mut matched_ids: HashSet<u32> = HashSet::new();

    for entry in entries {
        match match_entry(strings, entry, &matched_ids) {
            EntryOutcome::Applied(tier, idx) => {
                apply_match(
                    strings,
                    idx,
                    entry,
                    &mut result.updated_ids,
                    &mut matched_ids,
                );
                match tier {
                    MatchTier::Exact => result.tier_exact += 1,
                    MatchTier::Edid => result.tier_edid += 1,
                    MatchTier::Normalized => result.tier_normalized += 1,
                    MatchTier::Vocab => result.tier_vocab += 1,
                }
            }
            EntryOutcome::Ambiguous => result.ambiguous += 1,
            EntryOutcome::Unmatched => result.unmatched += 1,
        }
    }

    result
}

/// Convert XML entries and apply them through the shared matcher.
pub fn apply_xml_dictionary_entries(
    strings: &mut [SkyString],
    xml_entries: &[XmlStringEntry],
) -> MatchResult {
    let entries: Vec<DictionaryApplyEntry> = xml_entries
        .iter()
        .map(DictionaryApplyEntry::from_xml_entry)
        .collect();
    apply_dictionary_entries(strings, &entries)
}

/// Backward-compatible XML import entry point.
pub fn enhanced_import_match(
    strings: &mut [SkyString],
    xml_entries: &[XmlStringEntry],
) -> MatchResult {
    apply_xml_dictionary_entries(strings, xml_entries)
}

// ── 各层级查找逻辑 ──

fn match_entry(
    strings: &[SkyString],
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> EntryOutcome {
    match find_tier1(strings, entry, matched_ids) {
        TierMatch::Unique(idx) => return EntryOutcome::Applied(MatchTier::Exact, idx),
        TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
        TierMatch::None => {}
    }

    match find_tier2(strings, entry, matched_ids) {
        TierMatch::Unique(idx) => return EntryOutcome::Applied(MatchTier::Edid, idx),
        TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
        TierMatch::None => {}
    }

    match find_tier3(strings, entry, matched_ids) {
        TierMatch::Unique(idx) => return EntryOutcome::Applied(MatchTier::Normalized, idx),
        TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
        TierMatch::None => {}
    }

    match find_tier4(strings, entry, matched_ids) {
        TierMatch::Unique(idx) => EntryOutcome::Applied(MatchTier::Vocab, idx),
        TierMatch::Ambiguous => EntryOutcome::Ambiguous,
        TierMatch::None => EntryOutcome::Unmatched,
    }
}

/// Tier 1: exact triple match.
fn find_tier1(
    strings: &[SkyString],
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> TierMatch {
    let candidates: Vec<usize> = strings
        .iter()
        .enumerate()
        .filter(|(_, sk)| {
            !matched_ids.contains(&sk.id)
                && sk.esp_ptr.str_id == entry.str_id
                && sk.esp_ptr.record_sig == entry.record_sig
                && sk.esp_ptr.field_sig == entry.field_sig
        })
        .map(|(i, _)| i)
        .collect();

    match candidates.len() {
        0 => TierMatch::None,
        1 => TierMatch::Unique(candidates[0]),
        _ => TierMatch::Ambiguous,
    }
}

/// Tier 2: EDID hash match.
fn find_tier2(
    strings: &[SkyString],
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> TierMatch {
    let edid_hash = match entry.edid_hash {
        Some(hash) => hash,
        None => match entry.edid.as_ref() {
            Some(edid) if !edid.is_empty() => string_hash(edid),
            _ => return TierMatch::None,
        },
    };

    let candidates: Vec<usize> = strings
        .iter()
        .enumerate()
        .filter(|(_, sk)| {
            !matched_ids.contains(&sk.id)
                && sk.esp_ptr.edid_hash == edid_hash
                && sk.esp_ptr.record_sig == entry.record_sig
                && sk.esp_ptr.field_sig == entry.field_sig
        })
        .map(|(i, _)| i)
        .collect();

    match candidates.len() {
        0 => TierMatch::None,
        1 => TierMatch::Unique(candidates[0]),
        _ => disambiguate_by_normalized(strings, &candidates, &entry.source),
    }
}

/// Tier 3: normalized source match.
fn find_tier3(
    strings: &[SkyString],
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> TierMatch {
    let norm = normalization::normalize(&entry.source);
    if norm.is_empty() {
        return TierMatch::None;
    }
    let norm_hash = string_hash(&norm);

    let candidates: Vec<usize> = strings
        .iter()
        .enumerate()
        .filter(|(_, sk)| {
            !matched_ids.contains(&sk.id)
                && sk.esp_ptr.record_sig == entry.record_sig
                && sk.esp_ptr.field_sig == entry.field_sig
                && sk.normalized_hash == Some(norm_hash)
        })
        .map(|(i, _)| i)
        .collect();

    match candidates.len() {
        0 => TierMatch::None,
        1 => TierMatch::Unique(candidates[0]),
        _ => TierMatch::Ambiguous,
    }
}

/// Tier 4: vocabulary overlap.
fn find_tier4(
    strings: &[SkyString],
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> TierMatch {
    let entry_words: Vec<u32> = entry
        .source
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(string_hash)
        .collect();

    if entry_words.is_empty() {
        return TierMatch::None;
    }

    let entry_set: HashSet<u32> = entry_words.iter().copied().collect();

    let mut best_idx: Option<usize> = None;
    let mut best_score = 0.0f64;
    let mut best_score_count = 0u32;

    for (i, sk) in strings.iter().enumerate() {
        if matched_ids.contains(&sk.id) {
            continue;
        }
        if sk.esp_ptr.record_sig != entry.record_sig {
            continue;
        }
        if sk.esp_ptr.field_sig != entry.field_sig {
            continue;
        }
        if sk.word_hashes.is_empty() {
            continue;
        }

        let score = jaccard(&entry_set, &sk.word_hashes);
        if score < MIN_JACCARD {
            continue;
        }

        if score > best_score {
            best_score = score;
            best_idx = Some(i);
            best_score_count = 1;
        } else if (score - best_score).abs() < f64::EPSILON {
            best_score_count += 1;
        }
    }

    match (best_idx, best_score_count) {
        (Some(idx), 1) => TierMatch::Unique(idx),
        (Some(_), _) => TierMatch::Ambiguous,
        _ => TierMatch::None,
    }
}

// ── 辅助函数 ──

/// Apply a match to the target string.
fn apply_match(
    strings: &mut [SkyString],
    idx: usize,
    entry: &DictionaryApplyEntry,
    updated_ids: &mut Vec<u32>,
    matched_ids: &mut HashSet<u32>,
) {
    let sk = &mut strings[idx];

    if !entry.translation.is_empty() {
        sk.set_translation(entry.translation.clone());
    }

    if let Some(params) = entry.params {
        sk.params = params;
        if !sk.translation.is_empty() && !sk.params.is_translated() {
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
        }
    } else {
        sk.params.set(SkyStringParams::TRANSLATED, true);
        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, false);
    }

    updated_ids.push(sk.id);
    matched_ids.insert(sk.id);
}

/// Disambiguate EDID candidates using normalized source.
fn disambiguate_by_normalized(
    strings: &[SkyString],
    candidates: &[usize],
    source: &str,
) -> TierMatch {
    let norm = normalization::normalize(source);
    if norm.is_empty() {
        return TierMatch::Ambiguous;
    }
    let norm_hash = string_hash(&norm);

    let matching: Vec<usize> = candidates
        .iter()
        .filter(|&&i| strings[i].normalized_hash == Some(norm_hash))
        .copied()
        .collect();

    match matching.len() {
        1 => TierMatch::Unique(matching[0]),
        _ => TierMatch::Ambiguous,
    }
}

/// 计算词哈希集合的 Jaccard 相似度
///
/// Jaccard(A, B) = |A ∩ B| / |A ∪ B|
///
/// 参数 `entry_set` 预构建为 HashSet 以避免每次比较都重新构建。
fn jaccard(entry_set: &HashSet<u32>, sk_words: &[u32]) -> f64 {
    let sk_set: HashSet<u32> = sk_words.iter().copied().collect();

    let intersection = entry_set.intersection(&sk_set).count();
    let union = entry_set.len() + sk_set.len() - intersection;

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::esp_pointer::EspPointer;

    fn make_sk(
        id: u32,
        source: &str,
        str_id: i32,
        rec: [u8; 4],
        field: [u8; 4],
        edid_hash: u32,
    ) -> SkyString {
        let mut sk = SkyString::new(id, source.to_string(), String::new(), rec, field);
        sk.esp_ptr = EspPointer {
            str_id,
            form_id: 0,
            record_sig: rec,
            field_sig: field,
            index: 0,
            index_max: 0,
            edid_hash,
        };
        sk
    }

    fn make_entry(
        str_id: i32,
        edid: Option<&str>,
        rec: [u8; 4],
        field: [u8; 4],
        source: &str,
        translation: &str,
    ) -> XmlStringEntry {
        XmlStringEntry {
            list_index: 0,
            str_id,
            edid: edid.map(|s| s.to_string()),
            record_sig: rec,
            field_sig: field,
            index: 0,
            index_max: 0,
            source: source.to_string(),
            translation: translation.to_string(),
        }
    }

    #[test]
    fn test_tier1_exact_match() {
        let mut strings = vec![
            make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0),
            make_sk(1, "World", 8, *b"QUST", *b"NNAM", 0),
        ];
        let entries = vec![make_entry(1, None, *b"LCTN", *b"FULL", "Hello", "你好")];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.tier_exact, 1);
        assert_eq!(result.tier_edid, 0);
        assert_eq!(result.tier_vocab, 0);
        assert_eq!(result.tier_normalized, 0);
        assert_eq!(result.unmatched, 0);
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "你好");
        assert!(strings[0].params.is_translated());
        // 第二个字符串未被修改
        assert!(strings[1].translation.is_empty());
    }

    #[test]
    fn test_tier2_edid_match_different_str_id() {
        // 模拟跨版本 ESP：同一记录的 str_id 改变了
        let edid_hash = string_hash("TestLocation");
        let mut strings = vec![make_sk(
            0,
            "Hello World",
            999,
            *b"LCTN",
            *b"FULL",
            edid_hash,
        )];

        let entries = vec![make_entry(
            1, // 不同的 str_id
            Some("TestLocation"),
            *b"LCTN",
            *b"FULL",
            "Hello World",
            "你好世界",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.tier_exact, 0);
        assert_eq!(result.tier_edid, 1);
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "你好世界");
    }

    #[test]
    fn test_tier2_edid_no_match_without_edid() {
        let edid_hash = string_hash("TestLocation");
        let mut strings = vec![make_sk(0, "Hello", 999, *b"LCTN", *b"FULL", edid_hash)];

        // entry 没有 EDID → Tier 2 无法触发
        let entries = vec![make_entry(1, None, *b"LCTN", *b"FULL", "Hello", "你好")];

        let result = enhanced_import_match(&mut strings, &entries);

        // Tier 1 失败（str_id 不匹配），Tier 2 跳过（无 EDID），Tier 3 命中
        assert_eq!(result.tier_normalized, 1);
        assert_eq!(result.total_matched(), 1);
    }

    #[test]
    fn test_tier2_edid_disambiguate_by_normalized() {
        // 同一 EDID 对应两个字段（模拟 INFO 记录有多个 NAM1）
        let edid_hash = string_hash("TestQuest");
        let mut strings = vec![
            make_sk(0, "Retrieve the sword", 10, *b"INFO", *b"NAM1", edid_hash),
            make_sk(
                1,
                "Return to Jarl Balgruuf",
                11,
                *b"INFO",
                *b"NAM1",
                edid_hash,
            ),
        ];

        let entries = vec![make_entry(
            1,
            Some("TestQuest"),
            *b"INFO",
            *b"NAM1",
            "Return to Jarl Balgruuf",
            "回到白漫领主",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.tier_exact, 0);
        assert_eq!(result.tier_edid, 1);
        assert_eq!(result.total_matched(), 1);
        // 应该是 strings[1] 被匹配（规范化文本匹配消歧）
        assert_eq!(strings[1].translation, "回到白漫领主");
        assert!(strings[0].translation.is_empty());
    }

    #[test]
    fn test_tier2_edid_ambiguous_without_normalized_disambiguation() {
        let edid_hash = string_hash("TestQuest");
        let mut strings = vec![
            make_sk(0, "Alpha", 10, *b"INFO", *b"NAM1", edid_hash),
            make_sk(1, "Beta", 11, *b"INFO", *b"NAM1", edid_hash),
        ];

        let entries = vec![make_entry(
            1,
            Some("TestQuest"),
            *b"INFO",
            *b"NAM1",
            "Gamma",
            "回到白漫领主",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.tier_exact, 0);
        assert_eq!(result.tier_edid, 0);
        assert_eq!(result.ambiguous, 1);
        assert_eq!(result.total_matched(), 0);
        assert!(strings.iter().all(|sk| sk.translation.is_empty()));
    }

    #[test]
    fn test_tier3_vocab_overlap() {
        // 原文被扩展了（加了修饰词），但核心词汇重叠 > 50%
        let mut strings = vec![make_sk(
            0,
            "Retrieve the ancient sword from the tomb",
            999,
            *b"QUST",
            *b"NNAM",
            0,
        )];

        // "the" 出现两次但去重后仍然是 "the"
        // strings: {Retrieve, the, ancient, sword, from, tomb} = 6
        // entry:   {Retrieve, the, sword} = 3
        // Jaccard = 3/6 = 0.5 ≥ 0.5 ✓
        let entries = vec![make_entry(
            1,
            None,
            *b"QUST",
            *b"NNAM",
            "Retrieve the sword",
            "取回剑",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.tier_exact, 0);
        assert_eq!(result.tier_vocab, 1);
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "取回剑");
    }

    #[test]
    fn test_tier3_vocab_below_threshold() {
        // 词汇重叠不足 → 不匹配
        let mut strings = vec![make_sk(
            0,
            "Retrieve the ancient sword",
            999,
            *b"QUST",
            *b"NNAM",
            0,
        )];

        // "Find the key" vs "Retrieve the ancient sword": 只有 "the" 重叠 → Jaccard < 0.5
        let entries = vec![make_entry(
            1,
            None,
            *b"QUST",
            *b"NNAM",
            "Find the key",
            "找钥匙",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.total_matched(), 0);
        assert_eq!(result.unmatched, 1);
    }

    #[test]
    fn test_tier4_normalized_match() {
        // 标点、大小写不同，但规范化后完全一致
        let mut strings = vec![make_sk(0, "Hello, World!", 999, *b"LCTN", *b"FULL", 0)];

        let entries = vec![make_entry(
            1,
            None,
            *b"LCTN",
            *b"FULL",
            "hello world",
            "你好世界",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        // 规范化文本先于词汇重叠，因此这里应命中 Tier 3 normalized
        assert_eq!(result.tier_normalized, 1);
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "你好世界");
    }

    #[test]
    fn test_tier4_single_word_match() {
        // 单个词：词哈希相同但不同的 record → 不会错误匹配
        let mut strings = vec![
            make_sk(0, "Hello", 999, *b"LCTN", *b"FULL", 0),
            make_sk(1, "Hello", 888, *b"QUST", *b"NNAM", 0),
        ];

        let entries = vec![make_entry(
            1, None, *b"LCTN", *b"FULL", "hello", // 规范化后=hello，与 "Hello" 匹配
            "你好",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        // Tier 3 normalized should win before vocabulary overlap.
        assert_eq!(result.tier_normalized, 1);
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "你好");
        // strings[1] 的 REC+FIELD 不同，不应被匹配
        assert!(strings[1].translation.is_empty());
    }

    #[test]
    fn test_tier4_vocab_tie_is_ambiguous() {
        let mut strings = vec![
            make_sk(0, "Alpha Beta Gamma", 999, *b"QUST", *b"NNAM", 0),
            make_sk(1, "Gamma Alpha Beta", 998, *b"QUST", *b"NNAM", 0),
        ];

        let entries = vec![make_entry(
            1,
            None,
            *b"QUST",
            *b"NNAM",
            "Beta Gamma Alpha",
            "词汇翻译",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.total_matched(), 0);
        assert_eq!(result.ambiguous, 1);
        assert!(strings.iter().all(|sk| sk.translation.is_empty()));
    }

    #[test]
    fn test_sst_entry_preserves_params_and_uses_normalized_match() {
        let mut strings = vec![make_sk(0, "Hello, World!", 123, *b"LCTN", *b"FULL", 0)];

        let mut sst_entry = SkyString::new(
            99,
            "hello world".to_string(),
            "你好世界".to_string(),
            *b"LCTN",
            *b"FULL",
        );
        sst_entry.esp_ptr.str_id = 1;
        sst_entry.esp_ptr.record_sig = *b"LCTN";
        sst_entry.esp_ptr.field_sig = *b"FULL";
        sst_entry.params.set(SkyStringParams::TRANSLATED, true);
        sst_entry.params.set(SkyStringParams::VALIDATED, true);

        let entries = vec![DictionaryApplyEntry::from_sst_entry(&sst_entry)];
        let result = apply_dictionary_entries(&mut strings, &entries);

        assert_eq!(result.tier_normalized, 1);
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "你好世界");
        assert!(strings[0].params.is_translated());
        assert!(strings[0].params.is_validated());
        assert_eq!(strings[0].params, sst_entry.params);
    }

    #[test]
    fn test_no_double_match() {
        // 同一个 SkyString 不应被两个 entry 重复匹配
        let mut strings = vec![make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0)];

        let entries = vec![
            make_entry(1, None, *b"LCTN", *b"FULL", "Hello", "你好"),
            make_entry(1, None, *b"LCTN", *b"FULL", "Hello", "こんにちは"),
        ];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.total_matched(), 1); // 只匹配一次
        assert_eq!(result.unmatched, 1); // 第二次无法匹配
        assert_eq!(strings[0].translation, "你好"); // 保留第一次的翻译
    }

    #[test]
    fn test_all_tiers_integration() {
        // 综合测试：4 个 entry，各走不同 tier
        let edid_hash = string_hash("MyQuest");
        let mut strings = vec![
            // Tier 1: 精确匹配
            make_sk(0, "Hello", 10, *b"LCTN", *b"FULL", 0),
            // Tier 2: EDID 匹配（str_id 不同）
            make_sk(1, "World", 999, *b"QUST", *b"NNAM", edid_hash),
            // Tier 3: 词汇重叠（str_id 不同，无 EDID，词重叠 ≥ 50%）
            make_sk(
                2,
                "Retrieve the ancient sword from the tomb",
                888,
                *b"INFO",
                *b"NAM1",
                0,
            ),
            // 不匹配的
            make_sk(
                3,
                "Something completely different",
                777,
                *b"NPC_",
                *b"FULL",
                0,
            ),
        ];

        let entries = vec![
            make_entry(10, None, *b"LCTN", *b"FULL", "Hello", "你好"), // T1
            make_entry(1, Some("MyQuest"), *b"QUST", *b"NNAM", "World", "世界"), // T2 (str_id=1 vs 999)
            make_entry(2, None, *b"INFO", *b"NAM1", "Retrieve the sword", "取回剑"), // T3
            make_entry(3, None, *b"NPC_", *b"FULL", "Unrelated", "无关"),        // unmatched
        ];

        let result = enhanced_import_match(&mut strings, &entries);

        assert_eq!(result.tier_exact, 1);
        assert_eq!(result.tier_edid, 1);
        assert_eq!(result.tier_vocab, 1);
        assert_eq!(result.tier_normalized, 0);
        assert_eq!(result.unmatched, 1);
        assert_eq!(result.total_matched(), 3);

        assert_eq!(strings[0].translation, "你好");
        assert_eq!(strings[1].translation, "世界");
        assert_eq!(strings[2].translation, "取回剑");
        assert!(strings[3].translation.is_empty());
    }

    #[test]
    fn test_jaccard_identical() {
        let a: HashSet<u32> = [1, 2, 3].iter().copied().collect();
        let b = vec![1, 2, 3];
        assert!((jaccard(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_half_overlap() {
        let a: HashSet<u32> = [1, 2].iter().copied().collect();
        let b = vec![2, 3];
        assert!((jaccard(&a, &b) - 0.333).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a: HashSet<u32> = [1, 2].iter().copied().collect();
        let b = vec![3, 4];
        assert_eq!(jaccard(&a, &b), 0.0);
    }
}
