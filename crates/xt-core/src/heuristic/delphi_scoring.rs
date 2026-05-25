//! Delphi 风格的启发式搜索评分
//!
//! 移植自 Delphi 的 `TESVT_HeuristicSearch.pas` 和 `TESVT_TranslateFunc.pas`。
//! 用 Delphi 的六维复合评分替代了之前简单的 `normalized_similarity`，评分考虑了：
//! - 词级哈希与匹配
//! - 最长公共子串 (LCS)
//! - 最长公共前缀 (LCP)
//! - 别名标签 proxy 惩罚项
//! - 动态字数阈值
//!
//! # Delphi 评分原则
//! - 分数越低 = 匹配度越高 (0.0 = 完全相同)
//! - 分数 >= 阈值 = 被拒绝
//! - 默认阈值：根据字数而变化

use crate::types::esp_pointer::string_hash;

/// 来自 Delphi `TESVT_Const.pas:188` 的 Proxy 基础比例
const PROXYBASE_RATIO: f32 = 0.05;

/// 最大字数阈值上限 (Delphi `iLDMaxBreak`)
const LD_MAX_BREAK: u32 = 25;

/// 每个字符串提取的最大单词数 (Delphi `iWordThreshold`)
const WORD_THRESHOLD: usize = 1000;

// ── 单词分词 ──────────────────────────────────────────────────

/// 将文本拆分为单词（与 Delphi `getWordsMatchHash` 的分词匹配）。
///
/// 单词为小写，非字母数字字符用作分隔符。
fn tokenize_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if ch.is_ascii_uppercase() {
                current.push((ch as u8 + b'a' - b'A') as char);
            } else {
                for c in ch.to_lowercase() {
                    current.push(c);
                }
            }
        } else if !current.is_empty() {
            words.push(current);
            current = String::new();
            if words.len() >= WORD_THRESHOLD {
                break;
            }
        }
    }
    if !current.is_empty() {
        words.push(current);
    }

    words.truncate(WORD_THRESHOLD);
    words
}

/// 计算单词哈希（FNV-1a，与 Delphi `StringHash` 匹配）。
fn word_hashes(words: &[String]) -> Vec<u32> {
    words.iter().map(|w| string_hash(w)).collect()
}

// ── Proxy 惩罚 ──────────────────────────────────────────────────────

/// 计算字符串中 `<alias=...>` 标签的数量 (Delphi `GetStringProxy`)。
fn count_alias_tags(text: &str) -> u32 {
    let lower = text.to_lowercase();
    let mut count = 0u32;
    let mut pos = 0;

    loop {
        match lower[pos..].find("<alias=") {
            Some(offset) => {
                count += 1;
                pos += offset + 7; // skip past "<alias="
            }
            None => break,
        }
    }

    count
}

/// 计算 proxy 惩罚项：两个字符串之间别名标签数量的差值。
///
/// Delphi `GetStringProxy`：返回 proxy * proxybaseRatio。
/// 别名标签差异越大 = 惩罚越大 = 匹配分数越差。
fn alias_proxy_penalty(s1: &str, s2: &str) -> f32 {
    let a1 = count_alias_tags(s1);
    let a2 = count_alias_tags(s2);
    let diff = if a1 > a2 { a1 - a2 } else { a2 - a1 };
    diff as f32 * PROXYBASE_RATIO
}

// ── 阈值计算 ──────────────────────────────────────────────

/// Delphi `defineHeuristicThreshold` (TESVT_RegexUtils.pas:53-65)。
///
/// 基于字数的动态阈值：
/// - 0 个单词：0
/// - 1 个单词：1
/// - N 个单词：ceil(N/3) + 1，上限为 25
pub fn heuristic_threshold(word_count: usize) -> f32 {
    match word_count {
        0 => 0.0,
        1 => 1.0,
        n => {
            let t = (n as u32).div_ceil(3) + 1;
            t.min(LD_MAX_BREAK) as f32
        }
    }
}

/// Delphi `adjustHeuristicResult` (TESVT_RegexUtils.pas:67-78)。
///
/// 将原始 Levenshtein 距离调整为归一化分数。
/// 如果相对字数而言 LD 较小，则提升相似度。
pub fn adjust_heuristic_result(word_count: usize, ld_distance: f32) -> f32 {
    if ld_distance == 0.0 {
        return 0.0;
    }
    let tmp = (word_count / 15) as f32;
    if ld_distance <= tmp {
        0.55 + (ld_distance / 10.0)
    } else {
        ld_distance
    }
}

// ── 词级匹配 ────────────────────────────────────────────────

/// 计算两个单词哈希列表之间的词级 Levenshtein 距离。
///
/// 使用哈希比较而不是字符串比较（Delphi 也是如此）。
fn word_hash_levenshtein(hashes1: &[u32], hashes2: &[u32]) -> u32 {
    let n = hashes1.len();
    let m = hashes2.len();

    if n == 0 {
        return m as u32;
    }
    if m == 0 {
        return n as u32;
    }

    let mut prev = vec![0u32; m + 1];
    let mut curr = vec![0u32; m + 1];

    for j in 0..=m {
        prev[j] = j as u32;
    }

    for i in 1..=n {
        curr[0] = i as u32;
        for j in 1..=m {
            let cost = if hashes1[i - 1] == hashes2[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[m]
}

/// Delphi `getWordsMatchHash` (TESVT_TranslateFunc.pas:608-668)。
///
/// 分层词级匹配：
/// 1. 单词集合精确匹配 → 0.01 + proxy
/// 2. 长度相同，源字符串中不区分大小写匹配 → 0.3 + proxy
/// 3. 单词哈希 LD = 0 → 0.5
/// 4. 最终：调整后的 LD + proxy
///
/// 返回 (score, accepted)，其中 accepted = score <= result_threshold。
pub fn words_match_score(source: &str, candidate: &str, result_threshold: f32) -> (f32, bool) {
    let src_words = tokenize_words(source);
    let cand_words = tokenize_words(candidate);
    let proxy = alias_proxy_penalty(source, candidate);

    // 情况 1：精确字符串匹配（基于哈希）
    if string_hash(source) == string_hash(candidate) {
        let score = 0.01 + proxy;
        return (score, score <= result_threshold);
    }

    // 情况 2：相同长度，不区分大小写匹配
    if source.len() == candidate.len() {
        let src_lower = source.to_lowercase();
        let cand_lower = candidate.to_lowercase();
        if src_lower == cand_lower {
            let score = 0.3 + proxy;
            return (score, score <= result_threshold);
        }
    }

    // 情况 3：词级哈希 Levenshtein
    let src_hashes = word_hashes(&src_words);
    let cand_hashes = word_hashes(&cand_words);
    let mut ld = word_hash_levenshtein(&src_hashes, &cand_hashes) as f32;

    // 统计两个词列表（Delphi 使用最大词数作为阈值）
    let word_count = src_words.len().max(cand_words.len());

    // 调整 LD
    ld = adjust_heuristic_result(word_count, ld);

    // 如果 LD 在调整后为 0，推到 0.5
    if ld == 0.0 {
        ld = 0.5;
    }

    let score = ld + proxy;
    (score, score <= result_threshold)
}

// ── 子串匹配 ─────────────────────────────────────────────────

/// Delphi `getSubStringMatch` (TESVT_TranslateFunc.pas:581-596)。
///
/// 使用最长公共子串评分相似度：
/// - sSize = max(len1, len2) - lcs_size
/// - score = sSize * 0.1 + (如果 sSize == 0 则为 0.1 否则为 0.55) + proxy * 0.05
pub fn substring_match_score(source: &str, candidate: &str) -> f32 {
    let lcs_len = crate::heuristic::longest_common_substring_len(source, candidate);
    let max_len = source.len().max(candidate.len());
    let s_size = max_len.saturating_sub(lcs_len);
    let proxy = alias_proxy_penalty(source, candidate);

    let base = s_size as f32 * 0.1;
    let bonus = if s_size == 0 { 0.1 } else { 0.55 };
    base + bonus + proxy
}

// ── 最长公共前缀匹配 ─────────────────────────────────────

/// Delphi `getLongestCommonStrInt_Header` 变体评分。
///
/// 使用最长公共前缀评分相似度：
/// - 较高的 LCP = 较低的分数 = 较好的匹配
pub fn prefix_match_score(source: &str, candidate: &str) -> f32 {
    let lcp_len = crate::heuristic::longest_common_prefix_len(source, candidate);
    if lcp_len == 0 {
        return 10.0; // no match at prefix = very bad score
    }

    let max_len = source.len().max(candidate.len()) as f32;
    let lcp_ratio = lcp_len as f32 / max_len;
    let proxy = alias_proxy_penalty(source, candidate);

    // 缩放：1 - lcp_ratio + proxy
    (1.0 - lcp_ratio) * 2.0 + proxy
}

// ── Composite scoring ──────────────────────────────────────────────────

/// Delphi 风格组合启发式评分
///
/// 返回所有匹配策略中的最佳（最低）分数。
/// 分数代表"不相似度"：0.0 = 完全相同，越高越差。
pub fn delphi_heuristic_score(source: &str, candidate: &str, threshold: f32) -> f32 {
    // 首先尝试词级匹配（最可靠）
    let (word_score, word_accepted) = words_match_score(source, candidate, threshold);
    if word_accepted && word_score <= 0.3 {
        return word_score;
    }

    // 尝试子串匹配
    let sub_score = substring_match_score(source, candidate);

    // 尝试前缀匹配
    let prefix_score = prefix_match_score(source, candidate);

    // 返回最佳（最低）分数
    let best = word_score.min(sub_score).min(prefix_score);
    best
}

// ── 公共 API ─────────────────────────────────────────────────────────

/// Delphi 风格的启发式匹配结果。
#[derive(Clone, Debug)]
pub struct DelphiHeuristicMatch {
    pub source: String,
    pub translation: String,
    pub score: f32,        // 越低 = 越好
    pub word_score: f32,   // 词级分数
    pub sub_score: f32,    // 子串分数
    pub prefix_score: f32, // 前缀分数
}

/// 使用 Delphi 风格的评分搜索相似翻译。
///
/// 匹配 Delphi `TESVT_HeuristicSearch.pas` 的行为：
/// - 使用词级、子串和前缀匹配
/// - 返回按分数排序的结果（最低 = 最好）
/// - 按结果阈值过滤（Delphi 默认 ~0.5）
pub fn delphi_find_similar(
    source: &str,
    candidates: &[(String, String)],
    max_results: usize,
) -> Vec<DelphiHeuristicMatch> {
    let word_count = tokenize_words(source).len();
    let threshold = heuristic_threshold(word_count);

    let mut matches: Vec<DelphiHeuristicMatch> = candidates
        .iter()
        .filter(|(s, _)| !s.is_empty())
        .map(|(s, trans)| {
            let word_score = {
                let (score, _) = words_match_score(source, s, threshold);
                score
            };
            let sub_score = substring_match_score(source, s);
            let prefix_score = prefix_match_score(source, s);
            let best = word_score.min(sub_score).min(prefix_score);

            DelphiHeuristicMatch {
                source: s.clone(),
                translation: trans.clone(),
                score: best,
                word_score,
                sub_score,
                prefix_score,
            }
        })
        .collect();

    // 按分数升序排列（越低越好）
    matches.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    matches.truncate(max_results);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_words() {
        let words = tokenize_words("Hello World!");
        assert_eq!(words, vec!["hello", "world"]);

        let words = tokenize_words("Hello,   World!!");
        assert_eq!(words, vec!["hello", "world"]);

        let words = tokenize_words("Test123 abc");
        assert_eq!(words, vec!["test123", "abc"]);
    }

    #[test]
    fn test_count_alias_tags() {
        assert_eq!(count_alias_tags("Hello World"), 0);
        assert_eq!(count_alias_tags("<Alias=Player>"), 1);
        assert_eq!(count_alias_tags("<alias=Player> and <Alias=NPC>"), 2);
        assert_eq!(count_alias_tags("<ALIAS=target> <Alias=source>"), 2);
    }

    #[test]
    fn test_alias_proxy_penalty() {
        // 相同的别名数量 = 0 惩罚项
        assert_eq!(alias_proxy_penalty("<Alias=Player>", "<Alias=Player>"), 0.0);
        // 不同的数量 = 惩罚项
        let penalty = alias_proxy_penalty("<Alias=Player>", "No alias");
        assert!(penalty > 0.0);
        // Proxied 惩罚
        assert!((penalty - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_heuristic_threshold() {
        assert_eq!(heuristic_threshold(0), 0.0);
        assert_eq!(heuristic_threshold(1), 1.0);
        assert_eq!(heuristic_threshold(3), 2.0); // ceil(3/3)+1 = 2
        assert_eq!(heuristic_threshold(5), 3.0); // ceil(5/3)+1 = 3
        assert_eq!(heuristic_threshold(100), 25.0); // capped
    }

    #[test]
    fn test_adjust_heuristic_result() {
        // LD=0 保持为 0
        assert_eq!(adjust_heuristic_result(5, 0.0), 0.0);
        // LD=0.5 且有 15 个单词：floor(15/15)=1，LD<=1=true，结果=0.55+0.5/10=0.6
        assert!((adjust_heuristic_result(15, 0.5) - 0.6).abs() < 0.01);
        // LD=5 且有 3 个单词：floor(3/15)=0，LD>tmp，返回未改变的 LD
        assert_eq!(adjust_heuristic_result(3, 5.0), 5.0);
    }

    #[test]
    fn test_words_match_exact() {
        let (score, accepted) = words_match_score("Hello World", "Hello World", 0.5);
        assert!(accepted);
        assert!(score < 0.1); // 精确匹配 = 0.01
    }

    #[test]
    fn test_words_match_similar() {
        let (score, _) = words_match_score("Hello World", "Hello Earth", 0.5);
        // 应该相当低，因为 "Hello" 匹配
        assert!(score < 5.0);
    }

    #[test]
    fn test_words_match_different() {
        let (score, _) = words_match_score("Hello World", "Goodbye Universe", 0.5);
        // 不同的单词 = 更高的分数
        assert!(score > 0.5);
    }

    #[test]
    fn test_words_match_case_insensitive() {
        let (score, _) = words_match_score("hello world", "HELLO WORLD", 0.5);
        // 长度相同，不区分大小写匹配
        assert!(score <= 0.35); // 0.3 + 可能的 proxy 惩罚项
    }

    #[test]
    fn test_substring_match_score() {
        let score = substring_match_score("Hello World", "Hello Earth");
        let score2 = substring_match_score("Hello World", "Goodbye Universe");
        assert!(score < score2); // "Hello" 是公共子串
    }

    #[test]
    fn test_prefix_match_score() {
        let good = prefix_match_score("Hello World", "Hello Earth");
        let bad = prefix_match_score("Hello World", "Goodbye Universe");
        assert!(good < bad); // 公共前缀 "Hello " 产生更低的分数
    }

    #[test]
    fn test_delphi_find_similar_ordering() {
        let candidates = vec![
            ("Retrieve the sword".to_string(), "取回剑".to_string()),
            ("Find the key".to_string(), "找到钥匙".to_string()),
            ("Kill the dragon".to_string(), "杀死龙".to_string()),
            ("Speak to the Jarl".to_string(), "与领主对话".to_string()),
        ];

        let results = delphi_find_similar("Retrieve the axe", &candidates, 3);
        assert!(!results.is_empty());

        // "Retrieve the sword" 应该是第一个（最相似）
        assert_eq!(results[0].source, "Retrieve the sword");
        assert_eq!(results[0].translation, "取回剑");
    }

    #[test]
    fn test_word_hash_levenshtein() {
        let h1 = word_hashes(&tokenize_words("a b c"));
        let h2 = word_hashes(&tokenize_words("a b c"));
        assert_eq!(word_hash_levenshtein(&h1, &h2), 0);

        let h3 = word_hashes(&tokenize_words("a b d"));
        assert_eq!(word_hash_levenshtein(&h1, &h3), 1); // 一次替换
    }
}
