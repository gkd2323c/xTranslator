//! 启发式搜索算法模块
//!
//! 对应 Delphi TESVT_HeuristicSearch.pas
//! 用于在已翻译字符串中为未翻译字符串寻找可能的翻译参考
//!
//! 核心算法：
//! - Levenshtein 编辑距离：衡量字符串整体差异
//! - 最长公共子串（LCS）：衡量局部连续匹配  
//! - 最长公共前缀（LCP）：衡量开头部分匹配
//!
//! ## Delphi Scoring (delphi_scoring.rs)
//! Ports the original Delphi multi-dimensional scoring system including
//! word-level hashing, proxy penalties, and dynamic thresholds.

pub mod delphi_scoring;

/// 计算 Levenshtein 编辑距离
///
/// 返回将字符串 s 转换为字符串 t 所需的最少单字符编辑次数
/// （插入、删除、替换操作）
///
/// # 算法特点
/// - 使用滚动数组优化空间复杂度：只保留两行而非完整矩阵
/// - 时间复杂度：O(n*m)，空间复杂度：O(min(n,m))
///
/// # 参数
/// * `s` - 源字符串
/// * `t` - 目标字符串
///
/// # 返回
/// 最小编辑距离，值越小表示字符串越相似
pub fn levenshtein_distance(s: &str, t: &str) -> usize {
    let s_chars: Vec<char> = s.chars().collect();
    let t_chars: Vec<char> = t.chars().collect();
    let n = s_chars.len();
    let m = t_chars.len();

    // 边界情况：空字符串
    if n == 0 {
        return m; // 需要插入 m 个字符
    }
    if m == 0 {
        return n; // 需要删除 n 个字符
    }

    // 滚动数组优化：只保留前一行和当前行
    let mut prev = vec![0usize; m + 1]; // 前一行
    let mut curr = vec![0usize; m + 1]; // 当前行

    // 初始化第一行：空字符串到 t[0..j] 的编辑距离
    for j in 0..=m {
        prev[j] = j;
    }

    // 动态规划填充
    for i in 1..=n {
        curr[0] = i; // s[0..i] 到空字符串的编辑距离
        for j in 1..=m {
            // 如果字符相同，无需编辑；否则需要替换（成本为1）
            let cost = if s_chars[i - 1] == t_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1) // 删除操作：从 s 删除一个字符
                .min(curr[j - 1] + 1) // 插入操作：在 s 插入一个字符
                .min(prev[j - 1] + cost); // 替换操作（或匹配）
        }
        std::mem::swap(&mut prev, &mut curr); // 滚动：当前行变为前一行
    }

    prev[m] // 右下角即为最终编辑距离
}

/// 归一化相似度（0.0 ~ 1.0，1.0 表示完全相同）
pub fn normalized_similarity(s: &str, t: &str) -> f32 {
    let max_len = s.chars().count().max(t.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    let dist = levenshtein_distance(s, t);
    1.0 - (dist as f32 / max_len as f32)
}

/// 最长公共子串长度
pub fn longest_common_substring_len(s: &str, t: &str) -> usize {
    let s_chars: Vec<char> = s.chars().collect();
    let t_chars: Vec<char> = t.chars().collect();
    let n = s_chars.len();
    let m = t_chars.len();

    if n == 0 || m == 0 {
        return 0;
    }

    // 只保留两行
    let mut prev = vec![0usize; m];
    let mut curr = vec![0usize; m];
    let mut max_len = 0;

    for i in 0..n {
        for j in 0..m {
            if s_chars[i] == t_chars[j] {
                if i == 0 || j == 0 {
                    curr[j] = 1;
                } else {
                    curr[j] = prev[j - 1] + 1;
                }
                max_len = max_len.max(curr[j]);
            } else {
                curr[j] = 0;
            }
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    max_len
}

/// 最长公共前缀长度
pub fn longest_common_prefix_len(s: &str, t: &str) -> usize {
    s.chars().zip(t.chars()).take_while(|(a, b)| a == b).count()
}

/// 启发式匹配结果
#[derive(Clone, Debug)]
pub struct HeuristicMatch {
    pub source: String,
    pub translation: String,
    pub similarity: f32,    // 归一化相似度（0.0~1.0，越大越接近）
    pub levenshtein: usize, // 编辑距离（越小越接近）
    pub lcs_len: usize,     // 最长公共子串长度（越大通常越接近）
}

/// 在未翻译字符串中搜索与给定源文本最相似的已翻译条目
///
/// - `source`: 待搜索的源文本
/// - `candidates`: 候选翻译列表（源文本, 译文）
/// - `min_similarity`: 最小相似度阈值（默认 0.5）
/// - `max_results`: 最大返回结果数（默认 5）
pub fn find_similar_translations(
    source: &str,
    candidates: &[(String, String)],
    min_similarity: f32,
    max_results: usize,
) -> Vec<HeuristicMatch> {
    // 流程：
    // 1) 计算每个候选的多维相似指标
    // 2) 先按 similarity 阈值过滤
    // 3) 按 similarity 降序截断到 max_results
    let mut matches: Vec<HeuristicMatch> = candidates
        .iter()
        .filter(|(s, _)| !s.is_empty())
        .map(|(s, trans)| {
            let sim = normalized_similarity(source, s);
            let lev = levenshtein_distance(source, s);
            let lcs = longest_common_substring_len(source, s);
            HeuristicMatch {
                source: s.clone(),
                translation: trans.clone(),
                similarity: sim,
                levenshtein: lev,
                lcs_len: lcs,
            }
        })
        // 仅保留达到阈值的候选，避免把噪声结果返回给调用方。
        .filter(|m| m.similarity >= min_similarity)
        .collect();

    // 按相似度降序排序；若出现 NaN，按“相等”处理以保持稳定性。
    matches.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 限制返回条数，避免前端候选过多影响可读性。
    matches.truncate(max_results);
    matches
}

/// 使用 Delphi 风格评分进行启发式搜索（推荐）
///
/// 相比 `find_similar_translations`（字符级 Levenshtein），此函数使用
/// 词级哈希匹配、公共子串、公共前缀、代理惩罚等多维度评分，
/// 对齐 Delphi `TESVT_HeuristicSearch.pas` 的行为。
///
/// 返回结果按分数升序排列（分数越低越相似）。
/// 同时填充 `similarity` / `levenshtein` / `lcs_len` 字段
/// 以保持与 `HeuristicMatchDTO` 的兼容性。
pub fn find_similar_delphi(
    source: &str,
    candidates: &[(String, String)],
    max_results: usize,
) -> Vec<HeuristicMatch> {
    let delphi_results = delphi_scoring::delphi_find_similar(source, candidates, max_results);
    delphi_results
        .into_iter()
        .map(|dm| {
            let lev = levenshtein_distance(source, &dm.source);
            let lcs = longest_common_substring_len(source, &dm.source);
            // 将 Delphi 分数（越低越好）映射到 similarity（越高越好）
            let sim = if dm.score < 0.01 {
                1.0
            } else {
                (1.0 - (dm.score / 100.0).min(1.0)).max(0.0)
            };
            HeuristicMatch {
                source: dm.source,
                translation: dm.translation,
                similarity: sim,
                levenshtein: lev,
                lcs_len: lcs,
            }
        })
        .collect()
}

/// 批量搜索：为所有未翻译字符串找到最佳候选翻译
///
/// - `untranslated`: 未翻译字符串列表
/// - `translated`: 已翻译字符串列表（源文本, 译文）
/// - `min_similarity`: 最小相似度阈值
pub fn batch_heuristic_search(
    untranslated: &[(u32, String)], // (id, source)
    translated: &[(String, String)],
    min_similarity: f32,
) -> Vec<(u32, Vec<HeuristicMatch>)> {
    // 批量模式按“逐条独立搜索”实现，便于后续并行化替换（如 rayon）。
    untranslated
        .iter()
        .map(|(id, source)| {
            let matches = find_similar_translations(source, translated, min_similarity, 5);
            (*id, matches)
        })
        // 只返回命中结果，减少调用方后处理负担。
        .filter(|(_, m)| !m.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("kitten", ""), 6);
        assert_eq!(levenshtein_distance("", "sitting"), 7);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("Hello", "Hello"), 0);
        assert_eq!(levenshtein_distance("Hello", "Hella"), 1);
    }

    #[test]
    fn test_normalized_similarity() {
        assert!((normalized_similarity("Hello", "Hello") - 1.0).abs() < 0.001);
        assert!(normalized_similarity("Hello", "Hella") > 0.5);
        assert!(normalized_similarity("Hello", "World") < 0.5);
    }

    #[test]
    fn test_longest_common_substring() {
        assert_eq!(longest_common_substring_len("ABABC", "BABCA"), 4); // "BABC"
        assert_eq!(
            longest_common_substring_len("Hello World", "World Hello"),
            5
        ); // "World" or "Hello"
        assert_eq!(longest_common_substring_len("", "abc"), 0);
    }

    #[test]
    fn test_longest_common_prefix() {
        assert_eq!(longest_common_prefix_len("Hello World", "Hello There"), 6); // "Hello "
        assert_eq!(longest_common_prefix_len("abc", "def"), 0);
    }

    #[test]
    fn test_find_similar_translations() {
        let candidates = vec![
            ("Retrieve the sword".to_string(), "取回剑".to_string()),
            ("Find the key".to_string(), "找到钥匙".to_string()),
            ("Kill the dragon".to_string(), "杀死龙".to_string()),
            ("Speak to the Jarl".to_string(), "与领主对话".to_string()),
        ];

        let matches = find_similar_translations("Retrieve the axe", &candidates, 0.3, 3);
        assert!(!matches.is_empty());
        // “Retrieve the sword” 应该是最相似的
        assert_eq!(matches[0].source, "Retrieve the sword");
        assert_eq!(matches[0].translation, "取回剑");
        assert!(matches[0].similarity > 0.5);
    }

    #[test]
    fn test_batch_heuristic_search() {
        let untranslated = vec![
            (100u32, "Retrieve the axe".to_string()),
            (101u32, "Kill the bear".to_string()),
        ];

        let translated = vec![
            ("Retrieve the sword".to_string(), "取回剑".to_string()),
            ("Kill the dragon".to_string(), "杀死龙".to_string()),
        ];

        let results = batch_heuristic_search(&untranslated, &translated, 0.3);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 100);
        assert_eq!(results[0].1[0].translation, "取回剑");
    }
}
