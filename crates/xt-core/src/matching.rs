//! 共享字典应用匹配器
//!
//! XML 导入和 SST 加载共享相同的分层匹配顺序：
//!
//! | 层级 | 策略 | 关键字 | 置信度 |
//! |------|------|--------|--------|
//! | T1 | 精确三元组 | (str_id, record_sig, field_sig) | 非常高 |
//! | T2 | EDID 哈希 | (edid_hash, record_sig, field_sig) | 高 |
//! | T3 | 规范化源文本 | (normalized_hash, record_sig, field_sig) | 高 |
//! | T4 | 词汇重叠 | word_hashes Jaccard >= 0.5 | 中等 |
//!
//! 歧义匹配（多个候选项在同一层级）不会自动应用。
//!
//! 这是 xTranslator 的核心匹配算法，用于：
//! - SST 字典加载时的翻译匹配
//! - XML 导入时的翻译应用
//! - 启发式搜索的相似度计算

use std::collections::{HashMap, HashSet};

use crate::normalization;
use crate::types::esp_pointer::{string_hash, HeaderSig};
use crate::types::params::{SkyStringInternalParams, SkyStringParams};
use crate::types::sky_string::SkyString;
use crate::xml::XmlStringEntry;

/// 词汇重叠匹配的最小 Jaccard 阈值
///
/// Jaccard 相似度 = |A ∩ B| / |A ∪ B|
///
/// 阈值过低 → 误匹配风险增加（无关字符串可能被匹配）
/// 阈值过高 → 同义改写无法匹配（合法的改写被拒绝）
/// 0.5 意味着至少一半的规范化词汇需要重叠
///
/// 例如：
/// - "The Elder Scrolls" vs "Elder Scrolls" → Jaccard = 2/3 ≈ 0.67 ✓ 匹配
/// - "Hello World" vs "Goodbye World" → Jaccard = 1/3 ≈ 0.33 ✗ 不匹配
const MIN_JACCARD: f64 = 0.5;

/// 字典条目的源格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionarySourceFormat {
    /// 来自 XML 导入
    Xml,
    /// 来自 SST 字典
    Sst,
}

/// 中立的字典条目（被共享匹配器消费）
///
/// 这是 XML 和 SST 条目的统一表示，允许共享匹配逻辑。
#[derive(Debug, Clone)]
pub struct DictionaryApplyEntry {
    /// 条目来源（XML 或 SST）
    pub source_format: DictionarySourceFormat,
    /// Strings 文件类型索引：0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
    pub list_index: u8,
    /// Strings 文件中的字符串 ID（T1 匹配的关键）
    pub str_id: i32,
    /// 记录类型签名（如 "INFO", "DIAL"）
    pub record_sig: HeaderSig,
    /// 字段签名（如 "FULL", "DESC"）
    pub field_sig: HeaderSig,
    /// 字符串在 Strings 文件中的索引（用于 XXXX 处理）
    pub index: u16,
    /// 最大索引值（用于验证索引有效性）
    pub index_max: u16,
    /// 源文本（原文）
    pub source: String,
    /// 翻译文本（译文）
    pub translation: String,
    /// EDID（编辑器 ID，用于 T2 匹配）
    pub edid: Option<String>,
    /// EDID 的 FNV-1a 哈希值（T2 匹配的关键）
    pub edid_hash: Option<u32>,
    /// 状态参数（来自 SST 时有值）
    pub params: Option<SkyStringParams>,
    /// 协作 ID（用于多人协作）
    pub colab_id: u8,
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
            colab_id: 0,
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
            colab_id: entry.colab_id,
        }
    }

    pub fn to_old_data_sky_string(&self) -> SkyString {
        let mut sk = SkyString::new(
            0,
            self.source.clone(),
            self.translation.clone(),
            self.record_sig,
            self.field_sig,
        );
        sk.list_index = self.list_index;
        sk.colab_id = self.colab_id;
        sk.esp_ptr.str_id = self.str_id;
        sk.esp_ptr.record_sig = self.record_sig;
        sk.esp_ptr.field_sig = self.field_sig;
        sk.esp_ptr.index = self.index;
        sk.esp_ptr.index_max = self.index_max;
        sk.esp_ptr.edid_hash = self.edid_hash.unwrap_or(0);
        sk.params = self.params.unwrap_or_default();
        sk.params.set(SkyStringParams::OLD_DATA, true);
        sk.internal_params
            .set(SkyStringInternalParams::UNUSED_IN_SST, true);
        sk
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyPolicy {
    pub same_language: bool,
    pub tag_only: bool,
    pub replace_string_id: bool,
    pub preserve_old_data: bool,
}

impl ApplyPolicy {
    pub fn sst_load() -> Self {
        Self {
            preserve_old_data: true,
            ..Self::default()
        }
    }
}

impl Default for ApplyPolicy {
    fn default() -> Self {
        Self {
            same_language: false,
            tag_only: false,
            replace_string_id: false,
            preserve_old_data: false,
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
    /// Matched entries skipped because they are pending.
    pub pending_skipped: u32,
    /// SST entries preserved as old data for future saves.
    pub old_data_preserved: u32,
    /// Targets marked with warning because index cardinality was suspicious.
    pub warning: u32,
    /// Targets marked with bigWarning because index cardinality differed.
    pub big_warning: u32,
    /// Updated SkyString IDs.
    pub updated_ids: Vec<u32>,
    /// Unapplied SST entries retained for later save.
    pub old_data_entries: Vec<DictionaryApplyEntry>,
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
    Matched(MatchTier, usize),
    Ambiguous,
    Unmatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplyEffect {
    Applied,
    PendingSkipped,
}

type ExactKey = (i32, HeaderSig, HeaderSig);
type HashMatchKey = (u32, HeaderSig, HeaderSig);
type RecFieldKey = (HeaderSig, HeaderSig);

#[derive(Debug)]
struct MatchIndex {
    exact: HashMap<ExactKey, Vec<usize>>,
    edid: HashMap<HashMatchKey, Vec<usize>>,
    normalized: HashMap<HashMatchKey, Vec<usize>>,
    record_field: HashMap<RecFieldKey, Vec<usize>>,
    word_sets: Vec<HashSet<u32>>,
}

impl MatchIndex {
    fn build(strings: &[SkyString]) -> Self {
        let mut exact = HashMap::with_capacity(strings.len());
        let mut edid = HashMap::with_capacity(strings.len());
        let mut normalized = HashMap::with_capacity(strings.len());
        let mut record_field = HashMap::new();
        let mut word_sets = Vec::with_capacity(strings.len());

        for (idx, sk) in strings.iter().enumerate() {
            exact
                .entry((
                    sk.esp_ptr.str_id,
                    sk.esp_ptr.record_sig,
                    sk.esp_ptr.field_sig,
                ))
                .or_insert_with(Vec::new)
                .push(idx);

            edid.entry((
                sk.esp_ptr.edid_hash,
                sk.esp_ptr.record_sig,
                sk.esp_ptr.field_sig,
            ))
            .or_insert_with(Vec::new)
            .push(idx);

            if let Some(norm_hash) = sk.normalized_hash {
                normalized
                    .entry((norm_hash, sk.esp_ptr.record_sig, sk.esp_ptr.field_sig))
                    .or_insert_with(Vec::new)
                    .push(idx);
            }

            record_field
                .entry((sk.esp_ptr.record_sig, sk.esp_ptr.field_sig))
                .or_insert_with(Vec::new)
                .push(idx);

            word_sets.push(sk.word_hashes.iter().copied().collect());
        }

        Self {
            exact,
            edid,
            normalized,
            record_field,
            word_sets,
        }
    }

    fn exact_candidates(&self, key: ExactKey) -> &[usize] {
        self.exact.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn edid_candidates(&self, key: HashMatchKey) -> &[usize] {
        self.edid.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn normalized_candidates(&self, key: HashMatchKey) -> &[usize] {
        self.normalized.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn record_field_candidates(&self, key: RecFieldKey) -> &[usize] {
        self.record_field
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// Apply dictionary entries through the shared matcher.
pub fn apply_dictionary_entries(
    strings: &mut [SkyString],
    entries: &[DictionaryApplyEntry],
) -> MatchResult {
    apply_dictionary_entries_with_policy(strings, entries, ApplyPolicy::default())
}

pub fn apply_dictionary_entries_with_policy(
    strings: &mut [SkyString],
    entries: &[DictionaryApplyEntry],
    policy: ApplyPolicy,
) -> MatchResult {
    let mut result = MatchResult::default();
    let mut matched_ids: HashSet<u32> = HashSet::new();
    let index = MatchIndex::build(strings);

    for entry in entries {
        match match_entry(strings, &index, entry, &matched_ids) {
            EntryOutcome::Matched(tier, idx) => {
                let effect = apply_match(strings, idx, entry, tier, policy, &mut result);
                let matched_id = strings[idx].id;
                matched_ids.insert(matched_id);
                if effect == ApplyEffect::Applied {
                    match tier {
                        MatchTier::Exact => result.tier_exact += 1,
                        MatchTier::Edid => result.tier_edid += 1,
                        MatchTier::Normalized => result.tier_normalized += 1,
                        MatchTier::Vocab => result.tier_vocab += 1,
                    }
                } else {
                    result.pending_skipped += 1;
                }
            }
            EntryOutcome::Ambiguous => {
                result.ambiguous += 1;
                preserve_old_data(entry, policy, &mut result);
            }
            EntryOutcome::Unmatched => {
                result.unmatched += 1;
                preserve_old_data(entry, policy, &mut result);
            }
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
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> EntryOutcome {
    match find_tier1(strings, index, entry, matched_ids) {
        TierMatch::Unique(idx) => return EntryOutcome::Matched(MatchTier::Exact, idx),
        TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
        TierMatch::None => {}
    }

    match find_tier2(strings, index, entry, matched_ids) {
        TierMatch::Unique(idx) => return EntryOutcome::Matched(MatchTier::Edid, idx),
        TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
        TierMatch::None => {}
    }

    match find_tier3(strings, index, entry, matched_ids) {
        TierMatch::Unique(idx) => return EntryOutcome::Matched(MatchTier::Normalized, idx),
        TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
        TierMatch::None => {}
    }

    match find_tier4(strings, index, entry, matched_ids) {
        TierMatch::Unique(idx) => EntryOutcome::Matched(MatchTier::Vocab, idx),
        TierMatch::Ambiguous => EntryOutcome::Ambiguous,
        TierMatch::None => EntryOutcome::Unmatched,
    }
}

/// Tier 1: exact triple match.
fn find_tier1(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> TierMatch {
    single_unmatched_candidate(
        strings,
        index.exact_candidates((entry.str_id, entry.record_sig, entry.field_sig)),
        matched_ids,
    )
}

/// Tier 2: EDID hash match.
fn find_tier2(
    strings: &[SkyString],
    index: &MatchIndex,
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

    let candidates: Vec<usize> = index
        .edid_candidates((edid_hash, entry.record_sig, entry.field_sig))
        .iter()
        .copied()
        .filter(|&idx| !matched_ids.contains(&strings[idx].id))
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
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
) -> TierMatch {
    let norm = normalization::normalize(&entry.source);
    if norm.is_empty() {
        return TierMatch::None;
    }
    let norm_hash = string_hash(&norm);

    single_unmatched_candidate(
        strings,
        index.normalized_candidates((norm_hash, entry.record_sig, entry.field_sig)),
        matched_ids,
    )
}

/// Tier 4: vocabulary overlap.
fn find_tier4(
    strings: &[SkyString],
    index: &MatchIndex,
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

    for &i in index.record_field_candidates((entry.record_sig, entry.field_sig)) {
        let sk = &strings[i];
        if matched_ids.contains(&sk.id) {
            continue;
        }
        let sk_words = &index.word_sets[i];
        if sk_words.is_empty() {
            continue;
        }

        let score = jaccard_sets(&entry_set, sk_words);
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

fn single_unmatched_candidate(
    strings: &[SkyString],
    candidates: &[usize],
    matched_ids: &HashSet<u32>,
) -> TierMatch {
    let mut found = None;

    for &idx in candidates {
        if matched_ids.contains(&strings[idx].id) {
            continue;
        }
        if found.is_some() {
            return TierMatch::Ambiguous;
        }
        found = Some(idx);
    }

    match found {
        Some(idx) => TierMatch::Unique(idx),
        None => TierMatch::None,
    }
}

/// Apply a match to the target string.
fn apply_match(
    strings: &mut [SkyString],
    idx: usize,
    entry: &DictionaryApplyEntry,
    tier: MatchTier,
    policy: ApplyPolicy,
    result: &mut MatchResult,
) -> ApplyEffect {
    let sk = &mut strings[idx];
    let mut changed = false;

    if policy.replace_string_id && sk.esp_ptr.str_id != entry.str_id {
        sk.esp_ptr.str_id = entry.str_id;
        sk.internal_params
            .set(SkyStringInternalParams::STRING_ID_CHANGED, true);
        changed = true;
    }

    if policy.tag_only {
        if sk.colab_id != entry.colab_id {
            sk.colab_id = entry.colab_id;
            changed = true;
        }
        if changed {
            result.updated_ids.push(sk.id);
        }
        return ApplyEffect::Applied;
    }

    if entry.params.map(|p| p.is_pending()).unwrap_or(false) {
        if changed {
            result.updated_ids.push(sk.id);
        }
        return ApplyEffect::PendingSkipped;
    }

    if !entry.translation.is_empty() {
        sk.set_translation(entry.translation.clone());
        changed = true;
    }

    clear_warning_flags(&mut sk.internal_params);
    apply_status(sk, entry, policy);
    apply_index_warning(sk, entry, tier, result);

    if changed {
        result.updated_ids.push(sk.id);
    }

    ApplyEffect::Applied
}

fn preserve_old_data(entry: &DictionaryApplyEntry, policy: ApplyPolicy, result: &mut MatchResult) {
    if policy.preserve_old_data && entry.source_format == DictionarySourceFormat::Sst {
        result.old_data_preserved += 1;
        result.old_data_entries.push(entry.clone());
    }
}

fn apply_status(sk: &mut SkyString, entry: &DictionaryApplyEntry, policy: ApplyPolicy) {
    clear_translation_status(&mut sk.params);
    let params = entry.params.unwrap_or_default();

    if params.is_locked() {
        sk.params.set(SkyStringParams::LOCKED_TRANS, true);
    } else if params.is_incomplete() {
        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
    } else if policy.same_language {
        sk.params.set(SkyStringParams::VALIDATED, true);
    } else if !sk.translation.is_empty() {
        sk.params.set(SkyStringParams::TRANSLATED, true);
    } else {
        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
    }
}

fn apply_index_warning(
    sk: &mut SkyString,
    entry: &DictionaryApplyEntry,
    tier: MatchTier,
    result: &mut MatchResult,
) {
    if tier == MatchTier::Exact || (entry.index_max == 0 && sk.esp_ptr.index_max == 0) {
        return;
    }

    clear_translation_status(&mut sk.params);
    sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);

    if entry.index_max != sk.esp_ptr.index_max {
        sk.internal_params
            .set(SkyStringInternalParams::BIG_WARNING, true);
        result.big_warning += 1;
    } else {
        sk.internal_params
            .set(SkyStringInternalParams::WARNING, true);
        result.warning += 1;
    }
}

fn clear_translation_status(params: &mut SkyStringParams) {
    params.set(SkyStringParams::TRANSLATED, false);
    params.set(SkyStringParams::LOCKED_TRANS, false);
    params.set(SkyStringParams::INCOMPLETE_TRANS, false);
    params.set(SkyStringParams::VALIDATED, false);
}

fn clear_warning_flags(params: &mut SkyStringInternalParams) {
    params.set(SkyStringInternalParams::LOW_WARNING, false);
    params.set(SkyStringInternalParams::WARNING, false);
    params.set(SkyStringInternalParams::BIG_WARNING, false);
    params.set(SkyStringInternalParams::N_TRANS, false);
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
fn jaccard_sets(entry_set: &HashSet<u32>, sk_set: &HashSet<u32>) -> f64 {
    let intersection = entry_set.intersection(sk_set).count();
    let union = entry_set.len() + sk_set.len() - intersection;

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

#[cfg(test)]
fn jaccard(entry_set: &HashSet<u32>, sk_words: &[u32]) -> f64 {
    let sk_set: HashSet<u32> = sk_words.iter().copied().collect();

    jaccard_sets(entry_set, &sk_set)
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

    fn make_sst_entry(
        str_id: i32,
        rec: [u8; 4],
        field: [u8; 4],
        source: &str,
        translation: &str,
        params: SkyStringParams,
    ) -> DictionaryApplyEntry {
        let mut sk = SkyString::new(99, source.to_string(), translation.to_string(), rec, field);
        sk.esp_ptr.str_id = str_id;
        sk.esp_ptr.record_sig = rec;
        sk.esp_ptr.field_sig = field;
        sk.params = params;
        DictionaryApplyEntry::from_sst_entry(&sk)
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
        assert!(!strings[0].params.is_validated());
    }

    #[test]
    fn test_pending_sst_entry_does_not_overwrite_translation() {
        let mut strings = vec![make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0)];
        strings[0].set_translation("old".to_string());
        strings[0].params.set(SkyStringParams::TRANSLATED, true);

        let mut params = SkyStringParams::new();
        params.set(SkyStringParams::PENDING, true);
        let entries = vec![make_sst_entry(
            1, *b"LCTN", *b"FULL", "Hello", "new", params,
        )];

        let result = apply_dictionary_entries(&mut strings, &entries);

        assert_eq!(result.pending_skipped, 1);
        assert_eq!(result.total_matched(), 0);
        assert_eq!(strings[0].translation, "old");
        assert!(strings[0].params.is_translated());
    }

    #[test]
    fn test_locked_and_incomplete_sst_params_take_precedence() {
        let mut locked_strings = vec![make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0)];
        let mut locked_params = SkyStringParams::new();
        locked_params.set(SkyStringParams::LOCKED_TRANS, true);
        locked_params.set(SkyStringParams::TRANSLATED, true);
        locked_params.set(SkyStringParams::VALIDATED, true);
        let locked_entries = vec![make_sst_entry(
            1,
            *b"LCTN",
            *b"FULL",
            "Hello",
            "locked",
            locked_params,
        )];

        let locked_result = apply_dictionary_entries(&mut locked_strings, &locked_entries);

        assert_eq!(locked_result.total_matched(), 1);
        assert_eq!(locked_strings[0].translation, "locked");
        assert!(locked_strings[0].params.is_locked());
        assert!(!locked_strings[0].params.is_translated());
        assert!(!locked_strings[0].params.is_incomplete());
        assert!(!locked_strings[0].params.is_validated());

        let mut incomplete_strings = vec![make_sk(0, "World", 2, *b"LCTN", *b"FULL", 0)];
        let mut incomplete_params = SkyStringParams::new();
        incomplete_params.set(SkyStringParams::INCOMPLETE_TRANS, true);
        incomplete_params.set(SkyStringParams::TRANSLATED, true);
        let incomplete_entries = vec![make_sst_entry(
            2,
            *b"LCTN",
            *b"FULL",
            "World",
            "partial",
            incomplete_params,
        )];

        let incomplete_result =
            apply_dictionary_entries(&mut incomplete_strings, &incomplete_entries);

        assert_eq!(incomplete_result.total_matched(), 1);
        assert_eq!(incomplete_strings[0].translation, "partial");
        assert!(incomplete_strings[0].params.is_incomplete());
        assert!(!incomplete_strings[0].params.is_translated());
        assert!(!incomplete_strings[0].params.is_locked());
        assert!(!incomplete_strings[0].params.is_validated());
    }

    #[test]
    fn test_language_policy_controls_translated_vs_validated() {
        let mut params = SkyStringParams::new();
        params.set(SkyStringParams::TRANSLATED, true);
        let entries = vec![make_sst_entry(
            1, *b"LCTN", *b"FULL", "Hello", "你好", params,
        )];

        let mut different_language = vec![make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0)];
        let diff_result = apply_dictionary_entries_with_policy(
            &mut different_language,
            &entries,
            ApplyPolicy::default(),
        );
        assert_eq!(diff_result.total_matched(), 1);
        assert!(different_language[0].params.is_translated());
        assert!(!different_language[0].params.is_validated());

        let mut same_language = vec![make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0)];
        let same_result = apply_dictionary_entries_with_policy(
            &mut same_language,
            &entries,
            ApplyPolicy {
                same_language: true,
                ..ApplyPolicy::default()
            },
        );
        assert_eq!(same_result.total_matched(), 1);
        assert!(same_language[0].params.is_validated());
        assert!(!same_language[0].params.is_translated());
    }

    #[test]
    fn test_tag_only_and_string_id_replacement_policies() {
        let mut tag_strings = vec![make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0)];
        tag_strings[0].set_translation("old".to_string());
        tag_strings[0].params.set(SkyStringParams::TRANSLATED, true);

        let mut tag_entry = make_sst_entry(
            7,
            *b"LCTN",
            *b"FULL",
            "Hello",
            "new",
            SkyStringParams::new(),
        );
        tag_entry.colab_id = 42;

        let tag_result = apply_dictionary_entries_with_policy(
            &mut tag_strings,
            &[tag_entry.clone()],
            ApplyPolicy {
                tag_only: true,
                ..ApplyPolicy::default()
            },
        );

        assert_eq!(tag_result.total_matched(), 1);
        assert_eq!(tag_strings[0].translation, "old");
        assert!(tag_strings[0].params.is_translated());
        assert_eq!(tag_strings[0].colab_id, 42);
        assert_eq!(tag_strings[0].esp_ptr.str_id, 1);

        let mut id_strings = vec![make_sk(0, "Hello", 1, *b"LCTN", *b"FULL", 0)];
        let id_result = apply_dictionary_entries_with_policy(
            &mut id_strings,
            &[tag_entry],
            ApplyPolicy {
                replace_string_id: true,
                ..ApplyPolicy::default()
            },
        );

        assert_eq!(id_result.total_matched(), 1);
        assert_eq!(id_strings[0].esp_ptr.str_id, 7);
        assert!(id_strings[0]
            .internal_params
            .is_set(SkyStringInternalParams::STRING_ID_CHANGED));
    }

    #[test]
    fn test_index_max_warnings_for_fallback_matches() {
        let edid_hash = string_hash("TestQuest");
        let mut big_warning_strings = vec![make_sk(0, "Hello", 999, *b"INFO", *b"NAM1", edid_hash)];
        big_warning_strings[0].esp_ptr.index_max = 3;

        let mut big_entry = make_entry(1, Some("TestQuest"), *b"INFO", *b"NAM1", "Hello", "你好");
        big_entry.index_max = 2;

        let big_result = enhanced_import_match(&mut big_warning_strings, &[big_entry]);

        assert_eq!(big_result.tier_edid, 1);
        assert_eq!(big_result.big_warning, 1);
        assert!(big_warning_strings[0].params.is_incomplete());
        assert!(big_warning_strings[0]
            .internal_params
            .is_set(SkyStringInternalParams::BIG_WARNING));

        let mut warning_strings = vec![make_sk(0, "World", 999, *b"INFO", *b"NAM1", edid_hash)];
        warning_strings[0].esp_ptr.index_max = 2;
        let mut warning_entry =
            make_entry(1, Some("TestQuest"), *b"INFO", *b"NAM1", "World", "世界");
        warning_entry.index_max = 2;

        let warning_result = enhanced_import_match(&mut warning_strings, &[warning_entry]);

        assert_eq!(warning_result.tier_edid, 1);
        assert_eq!(warning_result.warning, 1);
        assert!(warning_strings[0].params.is_incomplete());
        assert!(warning_strings[0]
            .internal_params
            .is_set(SkyStringInternalParams::WARNING));
    }

    #[test]
    fn test_unmatched_and_ambiguous_sst_entries_are_preserved_as_old_data() {
        let entries = vec![make_sst_entry(
            1,
            *b"LCTN",
            *b"FULL",
            "Missing",
            "旧译文",
            SkyStringParams::new(),
        )];
        let mut strings = vec![make_sk(0, "Different", 2, *b"LCTN", *b"FULL", 0)];

        let result =
            apply_dictionary_entries_with_policy(&mut strings, &entries, ApplyPolicy::sst_load());

        assert_eq!(result.unmatched, 1);
        assert_eq!(result.old_data_preserved, 1);
        assert_eq!(result.old_data_entries.len(), 1);
        let old_data = result.old_data_entries[0].to_old_data_sky_string();
        assert!(old_data.params.is_old_data());

        let ambiguous_entries = vec![make_sst_entry(
            3,
            *b"INFO",
            *b"NAM1",
            "Shared",
            "旧共享译文",
            SkyStringParams::new(),
        )];
        let mut ambiguous_strings = vec![
            make_sk(0, "Shared A", 3, *b"INFO", *b"NAM1", 0),
            make_sk(1, "Shared B", 3, *b"INFO", *b"NAM1", 0),
        ];

        let ambiguous_result = apply_dictionary_entries_with_policy(
            &mut ambiguous_strings,
            &ambiguous_entries,
            ApplyPolicy::sst_load(),
        );

        assert_eq!(ambiguous_result.ambiguous, 1);
        assert_eq!(ambiguous_result.old_data_preserved, 1);
        assert_eq!(ambiguous_result.old_data_entries.len(), 1);
        assert!(ambiguous_strings.iter().all(|sk| sk.translation.is_empty()));
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
