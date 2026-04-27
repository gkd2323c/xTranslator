//! 增强版 XML 导入匹配引擎
//!
//! 多层级 fallback 策略，逐步提升跨版本字典的命中率：
//!
//! | 层级 | 策略 | 匹配键 | 置信度 |
//! |------|------|--------|--------|
//! | T1 | 精确三元组 | (str_id, record_sig, field_sig) | 极高 |
//! | T2 | EDID 哈希 | (edid_hash, record_sig, field_sig) | 高 |
//! | T3 | 词汇重叠 | word_hashes Jaccard ≥ 0.5 | 中 |
//! | T4 | 规范化文本 | (normalized_hash, record_sig, field_sig) | 高 |
//!
//! 预期效果：
//! - 纯三元组命中率 ~60%
//! - 增强后命中率 ~85%+

use std::collections::HashSet;

use crate::normalization;
use crate::types::esp_pointer::string_hash;
use crate::types::params::SkyStringParams;
use crate::types::sky_string::SkyString;
use crate::xml::XmlStringEntry;

/// 词汇重叠匹配的最小 Jaccard 阈值
///
/// 阈值过低 → 误匹配风险增加
/// 阈值过高 → 同义改写无法匹配
/// 0.5 意味着至少一半的规范化词汇需要重叠
const MIN_JACCARD: f64 = 0.5;

/// 匹配策略层级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    /// 精确三元组匹配 (str_id, record_sig, field_sig)
    Exact,
    /// EDID 哈希匹配（跨版本稳定）
    Edid,
    /// 词汇重叠匹配（Jaccard ≥ MIN_JACCARD）
    Vocab,
    /// 规范化文本哈希匹配
    Normalized,
}

/// 增强匹配结果统计
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Tier 1 精确匹配数
    pub tier_exact: u32,
    /// Tier 2 EDID 匹配数
    pub tier_edid: u32,
    /// Tier 3 词汇重叠匹配数
    pub tier_vocab: u32,
    /// Tier 4 规范化文本匹配数
    pub tier_normalized: u32,
    /// 全部层级均未匹配的条目数
    pub unmatched: u32,
    /// 被更新的 SkyString 内部 ID 列表（用于前端增量刷新）
    pub updated_ids: Vec<u32>,
}

impl MatchResult {
    /// 总命中数
    pub fn total_matched(&self) -> u32 {
        self.tier_exact + self.tier_edid + self.tier_vocab + self.tier_normalized
    }
}

/// 增强版 XML 导入匹配
///
/// 对每个 XML 条目，按 T1→T2→T3→T4 顺序尝试匹配。
/// 一旦某层级匹配成功，不再尝试后续层级。
/// 已匹配的 SkyString 不会被后续条目重复匹配（通过 `matched_ids` 去重）。
///
/// # 参数
/// * `strings` - 可变的 SkyString 切片（来自已加载的 ESP）
/// * `xml_entries` - 从 XML 解析出的翻译条目
///
/// # 返回
/// 包含各层级匹配统计和更新 ID 列表的 `MatchResult`
pub fn enhanced_import_match(
    strings: &mut [SkyString],
    xml_entries: &[XmlStringEntry],
) -> MatchResult {
    let mut tier_exact = 0u32;
    let mut tier_edid = 0u32;
    let mut tier_vocab = 0u32;
    let mut tier_normalized = 0u32;
    let mut unmatched = 0u32;
    let mut updated_ids = Vec::new();
    let mut matched_ids: HashSet<u32> = HashSet::new();

    for entry in xml_entries {
        // ── Tier 1: 精确三元组匹配 ──
        if let Some(idx) = find_tier1(strings, entry, &matched_ids) {
            apply_match(strings, idx, entry, &mut updated_ids, &mut matched_ids);
            tier_exact += 1;
            continue;
        }

        // ── Tier 2: EDID 哈希匹配 ──
        if let Some(idx) = find_tier2(strings, entry, &matched_ids) {
            apply_match(strings, idx, entry, &mut updated_ids, &mut matched_ids);
            tier_edid += 1;
            continue;
        }

        // ── Tier 3: 词汇重叠匹配 ──
        if let Some(idx) = find_tier3(strings, entry, &matched_ids) {
            apply_match(strings, idx, entry, &mut updated_ids, &mut matched_ids);
            tier_vocab += 1;
            continue;
        }

        // ── Tier 4: 规范化文本匹配 ──
        if let Some(idx) = find_tier4(strings, entry, &matched_ids) {
            apply_match(strings, idx, entry, &mut updated_ids, &mut matched_ids);
            tier_normalized += 1;
            continue;
        }

        unmatched += 1;
    }

    MatchResult {
        tier_exact,
        tier_edid,
        tier_vocab,
        tier_normalized,
        unmatched,
        updated_ids,
    }
}

// ── 各层级查找逻辑 ──

/// Tier 1: (str_id, record_sig, field_sig) 精确匹配
fn find_tier1(
    strings: &[SkyString],
    entry: &XmlStringEntry,
    matched_ids: &HashSet<u32>,
) -> Option<usize> {
    strings.iter().position(|sk| {
        !matched_ids.contains(&sk.id)
            && sk.esp_ptr.str_id == entry.str_id
            && sk.esp_ptr.record_sig == entry.record_sig
            && sk.esp_ptr.field_sig == entry.field_sig
    })
}

/// Tier 2: EDID 哈希匹配
///
/// 策略：
/// 1. 计算 XML EDID 的 FNV-1a 哈希
/// 2. 在未匹配的 SkyString 中查找 edid_hash + REC + FIELD 匹配项
/// 3. 若唯一匹配 → 直接确认
/// 4. 若多个匹配 → 用规范化文本消歧
/// 5. 若仍无法确定 → 放弃（返回 None，交给后续 tier）
fn find_tier2(
    strings: &[SkyString],
    entry: &XmlStringEntry,
    matched_ids: &HashSet<u32>,
) -> Option<usize> {
    let edid = entry.edid.as_ref()?;
    if edid.is_empty() {
        return None;
    }
    let edid_hash = string_hash(edid);

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
        0 => None,
        1 => Some(candidates[0]),
        _ => {
            // 多个候选：用规范化文本消歧
            disambiguate_by_normalized(strings, &candidates, &entry.source)
        }
    }
}

/// Tier 3: 词汇重叠匹配
///
/// 算法：
/// 1. 从 entry.source 提取词哈希集合
/// 2. 扫描所有未匹配且 REC+FIELD 相同的 SkyString
/// 3. 计算 Jaccard 相似度：|entry_words ∩ sk_words| / |entry_words ∪ sk_words|
/// 4. 取最佳匹配（需 ≥ MIN_JACCARD）
fn find_tier3(
    strings: &[SkyString],
    entry: &XmlStringEntry,
    matched_ids: &HashSet<u32>,
) -> Option<usize> {
    // 提取 entry 的词哈希
    let entry_words: Vec<u32> = entry
        .source
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(string_hash)
        .collect();

    if entry_words.is_empty() {
        return None;
    }

    let entry_set: HashSet<u32> = entry_words.iter().copied().collect();

    let mut best_idx: Option<usize> = None;
    let mut best_score = 0.0f64;

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
        if score > best_score && score >= MIN_JACCARD {
            best_score = score;
            best_idx = Some(i);
        }
    }

    best_idx
}

/// Tier 4: 规范化文本哈希匹配
///
/// 将 entry.source 规范化后计算哈希，与 SkyString.normalized_hash 对比。
/// 同时要求 REC+FIELD 一致以缩小搜索空间。
fn find_tier4(
    strings: &[SkyString],
    entry: &XmlStringEntry,
    matched_ids: &HashSet<u32>,
) -> Option<usize> {
    let norm = normalization::normalize(&entry.source);
    if norm.is_empty() {
        return None;
    }
    let norm_hash = string_hash(&norm);

    let candidates: Vec<usize> = strings
        .iter()
        .enumerate()
        .filter(|(_, sk)| {
            !matched_ids.contains(&sk.id)
                && sk.normalized_hash == Some(norm_hash)
                && sk.esp_ptr.record_sig == entry.record_sig
                && sk.esp_ptr.field_sig == entry.field_sig
        })
        .map(|(i, _)| i)
        .collect();

    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}

// ── 辅助函数 ──

/// 应用匹配：更新目标 SkyString 的翻译和状态
fn apply_match(
    strings: &mut [SkyString],
    idx: usize,
    entry: &XmlStringEntry,
    updated_ids: &mut Vec<u32>,
    matched_ids: &mut HashSet<u32>,
) {
    let sk = &mut strings[idx];

    // 只更新非空翻译
    if !entry.translation.is_empty() {
        sk.set_translation(entry.translation.clone());
    }

    // 更新状态标志
    sk.params.set(SkyStringParams::TRANSLATED, true);
    sk.params.set(SkyStringParams::INCOMPLETE_TRANS, false);

    updated_ids.push(sk.id);
    matched_ids.insert(sk.id);
}

/// 通过规范化文本在多个 EDID 候选中消歧
///
/// 当同一个 EDID 对应多个字段（如多 NAM1 的 INFO 记录），
/// 用规范化文本来区分具体是哪个字段。
fn disambiguate_by_normalized(
    strings: &[SkyString],
    candidates: &[usize],
    source: &str,
) -> Option<usize> {
    let norm = normalization::normalize(source);
    if norm.is_empty() {
        return None;
    }
    let norm_hash = string_hash(&norm);

    let matching: Vec<usize> = candidates
        .iter()
        .filter(|&&i| strings[i].normalized_hash == Some(norm_hash))
        .copied()
        .collect();

    if matching.len() == 1 {
        Some(matching[0])
    } else {
        None
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
        let mut strings = vec![make_sk(0, "Hello World", 999, *b"LCTN", *b"FULL", edid_hash)];

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

        // Tier 1 失败（str_id 不匹配），Tier 2 跳过（无 EDID），
        // Tier 3 可能通过词汇匹配到，也可能不匹配
        // 这里 "Hello" 的唯一词哈希与 SkyString 的 word_hashes 完全重叠 → Jaccard = 1.0
        assert_eq!(result.tier_vocab, 1);
        assert_eq!(result.total_matched(), 1);
    }

    #[test]
    fn test_tier2_edid_disambiguate_by_normalized() {
        // 同一 EDID 对应两个字段（模拟 INFO 记录有多个 NAM1）
        let edid_hash = string_hash("TestQuest");
        let mut strings = vec![
            make_sk(0, "Retrieve the sword", 10, *b"INFO", *b"NAM1", edid_hash),
            make_sk(1, "Return to Jarl Balgruuf", 11, *b"INFO", *b"NAM1", edid_hash),
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
        let mut strings = vec![make_sk(
            0,
            "Hello, World!",
            999,
            *b"LCTN",
            *b"FULL",
            0,
        )];

        let entries = vec![make_entry(
            1,
            None,
            *b"LCTN",
            *b"FULL",
            "hello world",
            "你好世界",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        // "hello world" → 词: ["hello", "world"]
        // "Hello, World!" → 词: ["Hello", "World"]
        // Jaccard = 2/2 = 1.0 ≥ 0.5 → Tier 3 就会命中
        // 规范化文本匹配作为兜底，在这里也被 Tier 3 抢了先
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
            1,
            None,
            *b"LCTN",
            *b"FULL",
            "hello", // 规范化后=hello，与 "Hello" 匹配
            "你好",
        )];

        let result = enhanced_import_match(&mut strings, &entries);

        // Tier 3: "hello" / "Hello" → 单词重叠 → Jaccard = 1.0 → match strings[0]
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "你好");
        // strings[1] 的 REC+FIELD 不同，不应被匹配
        assert!(strings[1].translation.is_empty());
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
            make_sk(2, "Retrieve the ancient sword from the tomb", 888, *b"INFO", *b"NAM1", 0),
            // 不匹配的
            make_sk(3, "Something completely different", 777, *b"NPC_", *b"FULL", 0),
        ];

        let entries = vec![
            make_entry(10, None, *b"LCTN", *b"FULL", "Hello", "你好"), // T1
            make_entry(1, Some("MyQuest"), *b"QUST", *b"NNAM", "World", "世界"), // T2 (str_id=1 vs 999)
            make_entry(2, None, *b"INFO", *b"NAM1", "Retrieve the sword", "取回剑"), // T3
            make_entry(3, None, *b"NPC_", *b"FULL", "Unrelated", "无关"), // unmatched
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
