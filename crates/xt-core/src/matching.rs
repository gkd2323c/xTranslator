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
    /// 记录 FormID（SST V4 匹配模式的主键；XML 导入时为 0）
    pub form_id: u32,
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
    /// 从 XML 导入条目构造字典应用条目
    pub fn from_xml_entry(entry: &XmlStringEntry) -> Self {
        Self {
            source_format: DictionarySourceFormat::Xml,
            list_index: entry.list_index,
            str_id: entry.str_id,
            form_id: 0,
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

    /// 从 SST SkyString 条目构造字典应用条目
    pub fn from_sst_entry(entry: &SkyString) -> Self {
        Self {
            source_format: DictionarySourceFormat::Sst,
            list_index: entry.list_index,
            str_id: entry.esp_ptr.str_id,
            form_id: entry.esp_ptr.form_id,
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

    /// 将当前条目转换为标记了 `OLD_DATA` 的 SkyString
    ///
    /// 用于 SST 加载时保留不能匹配的历史条目（如 `preserve_old_data` 策略开启时）。
    /// 返回的 SkyString 将设置 `SkyStringParams::OLD_DATA` 和 `UNUSED_IN_SST` 标志。
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
        sk.esp_ptr.form_id = self.form_id;
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

/// SST 字典覆盖范围（Delphi Form12 RadioGroup1）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SstOverwriteScope {
    /// 0: 全部字符串（未锁定项）
    #[default]
    All = 0,
    /// 1: 仅未翻译项（未翻译且未验证）
    NoTransExclusive = 1,
    /// 2: 严格未翻译项（保留 Delphi 原名；排除 incompleteTrans）
    NoTransAndPartial = 2,
    /// 3: 仅部分翻译项
    PartialOnly = 3,
    /// 4: 仅选中项
    Selection = 4,
}

/// SST 字典匹配模式（Delphi Form12 RadioGroup2）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SstMatchMode {
    /// 0: 普通字符串使用 FormID + EDID hash + field + index（V4Edid）；VMAD 使用 V4Strict
    FormIdOnly = 0,
    /// 1: FormID + EDID hash + 严格源文本 + field + index（Delphi V4Strict）
    #[default]
    FormIdStrictString = 1,
    /// 2: 普通字符串使用 V4Relax；VMAD 仍使用 FormID + EDID hash + field + 源文 + index（V4Strict）
    FormIdRelaxedString = 2,
    /// 3: 仅源文本精确一致（忽略 FormID；重复项按 REC/FIELD 消歧）
    StringOnly = 3,
}

/// SST 字典应用高级选项（对齐 Delphi TESVT_ApplySSTOpts）
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SstApplyOptions {
    /// 覆盖范围（5 种）
    pub overwrite_scope: SstOverwriteScope,
    /// 匹配模式（4 种）
    pub match_mode: SstMatchMode,
    /// 仅打标模式（不覆盖文本，仅标记匹配状态）
    pub tag_only: bool,
    /// 匹配前重置候选字符串状态；未命中候选也会保持重置状态
    pub reset_state: bool,
    /// 限制在过滤结果范围内
    pub restrict_to_filter: bool,
    /// 当前选中的字符串 ID 列表（用于 Selection 范围）
    pub selected_ids: Option<Vec<u32>>,
    /// 当前过滤可见的字符串 ID 列表（用于 restrict_to_filter）
    pub filtered_ids: Option<Vec<u32>>,
}

/// 字典应用策略（控制匹配行为的多个选项）
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyPolicy {
    /// 同语言模式：为 true 时仅应用来自相同语言的条目
    pub same_language: bool,
    /// 仅打标模式：为 true 时不写入翻译文本，仅标记匹配状态
    pub tag_only: bool,
    /// 替换 str_id：为 true 时用来源条目的 str_id 覆盖目标条目的 str_id
    pub replace_string_id: bool,
    /// 保留旧数据：为 true 时将未匹配/歧义条目以 OLD_DATA 标志保存，用于 SST 加载时保留历史数据
    pub preserve_old_data: bool,
    /// SST 高级选项（如果提供，将覆盖默认行为）
    pub sst_options: Option<SstApplyOptions>,
}

impl ApplyPolicy {
    /// 创建 SST 加载模式的策略：开启 `preserve_old_data`，将未匹配的 SST 条目保存为 OLD_DATA
    pub fn sst_load() -> Self {
        Self {
            preserve_old_data: true,
            ..Self::default()
        }
    }

    /// 创建带自定义 SST 高级选项的策略
    pub fn sst_load_with_options(options: SstApplyOptions) -> Self {
        let tag_only = options.tag_only;
        Self {
            preserve_old_data: true,
            tag_only,
            sst_options: Some(options),
            ..Self::default()
        }
    }
}

/// 匹配层级（Tier）枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    /// T1 精确三元组匹配 (str_id, record_sig, field_sig)。
    Exact,
    /// EDID 哈希匹配。
    Edid,
    /// 规范化源文本匹配。
    Normalized,
    /// 词汇重叠匹配。
    Vocab,
}

/// 共享字典应用的结果。
#[derive(Debug, Clone, Default)]
pub struct MatchResult {
    /// Tier 1 精确匹配数。
    pub tier_exact: u32,
    /// Tier 2 EDID 匹配数。
    pub tier_edid: u32,
    /// Tier 3 规范化源文本匹配数。
    pub tier_normalized: u32,
    /// Tier 4 词汇重叠匹配数。
    pub tier_vocab: u32,
    /// 未自动应用的歧义条目数。
    pub ambiguous: u32,
    /// 未匹配的条目数。
    pub unmatched: u32,
    /// 因处于 pending 状态而被跳过的已匹配条目数。
    pub pending_skipped: u32,
    /// 保存为旧数据以备后用且保留的 SST 条目数。
    pub old_data_preserved: u32,
    /// 因索引基数可疑而被标记为警告（warning）的目标数。
    pub warning: u32,
    /// 因索引基数不一致而被标记为大警告（bigWarning）的目标数。
    pub big_warning: u32,
    /// 更新后的 SkyString ID。
    pub updated_ids: Vec<u32>,
    /// 为以后保存而保留的未应用 SST 条目。
    pub old_data_entries: Vec<DictionaryApplyEntry>,
}

impl MatchResult {
    /// 返回所有层级命中数之和（T1+T2+T3+T4）。
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
type FormIdKey = (u32, u32);

#[derive(Debug)]
struct MatchIndex {
    exact: HashMap<ExactKey, Vec<usize>>,
    edid: HashMap<HashMatchKey, Vec<usize>>,
    normalized: HashMap<HashMatchKey, Vec<usize>>,
    record_field: HashMap<RecFieldKey, Vec<usize>>,
    form_id: HashMap<FormIdKey, Vec<usize>>,
    source_hash: HashMap<u32, Vec<usize>>,
    word_sets: Vec<HashSet<u32>>,
}

impl MatchIndex {
    fn build(strings: &[SkyString]) -> Self {
        let mut exact = HashMap::with_capacity(strings.len());
        let mut edid = HashMap::with_capacity(strings.len());
        let mut normalized = HashMap::with_capacity(strings.len());
        let mut record_field = HashMap::new();
        let mut form_id = HashMap::with_capacity(strings.len());
        let mut source_hash = HashMap::with_capacity(strings.len());
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

            form_id
                .entry((sanitize_form_id(sk.esp_ptr.form_id), sk.esp_ptr.edid_hash))
                .or_insert_with(Vec::new)
                .push(idx);

            source_hash
                .entry(sk.hash)
                .or_insert_with(Vec::new)
                .push(idx);

            word_sets.push(sk.word_hashes.iter().copied().collect());
        }

        Self {
            exact,
            edid,
            normalized,
            record_field,
            form_id,
            source_hash,
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

    fn form_id_candidates(&self, key: FormIdKey) -> &[usize] {
        self.form_id.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn source_candidates(&self, hash: u32) -> &[usize] {
        self.source_hash
            .get(&hash)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// Delphi `sanitizeFormID(formID, $01)` 的等价实现。
///
/// 比较 SST V4 FormID 时会把普通 master 的高字节统一为 1；
/// Starfield 的 light/medium master 分别规范化其索引位。
fn sanitize_form_id(form_id: u32) -> u32 {
    let high = (form_id >> 24) as u8;
    if high == 0 {
        form_id
    } else if high == 0xFE {
        if ((form_id >> 12) & 0xFF) == 0 {
            form_id
        } else {
            (form_id & !(0xFFF << 12)) | (1 << 12)
        }
    } else if high == 0xFD {
        if ((form_id >> 16) & 0xFF) == 0 {
            form_id
        } else {
            (form_id & !(0xFF << 16)) | (1 << 16)
        }
    } else {
        (form_id & 0x00FF_FFFF) | (1 << 24)
    }
}

/// 通过共享匹配器应用字典条目。
pub fn apply_dictionary_entries(
    strings: &mut [SkyString],
    entries: &[DictionaryApplyEntry],
) -> MatchResult {
    apply_dictionary_entries_with_policy(strings, entries, ApplyPolicy::default())
}

/// 判断条目是否是 VMAD 脚本字符串。
///
/// 正常 ESP 解析会设置 `IS_VMAD_STRING`；负的 VMAD 偏移和 `VMAD` 字段
/// 同时作为兼容旧缓存/反序列化条目的结构性兜底。
fn is_vmad_string(sk: &SkyString) -> bool {
    sk.internal_params
        .is_set(SkyStringInternalParams::IS_VMAD_STRING)
        || (sk.esp_ptr.str_id < 0 && sk.esp_ptr.field_sig == *b"VMAD")
}

/// 检查候选目标是否在覆盖范围和过滤范围内
fn is_candidate_eligible(
    sk: &SkyString,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
) -> bool {
    let is_vmad = is_vmad_string(sk);
    // 普通字符串由通用 comparator 排除 lockedTrans；SST 的专用 VMAD
    // comparator 只判断 VMAD 与 scope，允许 locked VMAD 进入匹配。
    let locked_vmad_is_sst_target = is_vmad && policy.sst_options.is_some();
    if (sk.params.is_locked() && !locked_vmad_is_sst_target)
        || sk
            .internal_params
            .is_set(SkyStringInternalParams::PEX_NO_TRANS)
    {
        return false;
    }

    if let Some(opts) = &policy.sst_options {
        // Delphi VMAD 特殊保护逻辑 (getfProcCompareOptVMADString):
        // 在 StringOnly 模式下，All / NoTransExclusive / NoTransAndPartial 对 VMAD 脚本字符串直接屏蔽 (compareOptBlock)，
        // 仅允许在 PartialOnly 或 Selection 显式指定的目标范围内应用！
        if is_vmad && opts.match_mode == SstMatchMode::StringOnly {
            match opts.overwrite_scope {
                SstOverwriteScope::All
                | SstOverwriteScope::NoTransExclusive
                | SstOverwriteScope::NoTransAndPartial => {
                    return false;
                }
                SstOverwriteScope::PartialOnly | SstOverwriteScope::Selection => {}
            }
        }

        // 限制在过滤结果范围内
        if opts.restrict_to_filter {
            match filtered_set {
                Some(f_set) if f_set.contains(&sk.id) => {}
                _ => return false,
            }
        }

        // 覆盖范围判断
        match opts.overwrite_scope {
            SstOverwriteScope::All => {}
            SstOverwriteScope::NoTransExclusive => {
                // 仅未翻译且未验证项
                if sk.params.is_translated() || sk.params.is_validated() {
                    return false;
                }
            }
            SstOverwriteScope::NoTransAndPartial => {
                // Delphi 名称虽叫 NoTransAndPartials，实际比较器明确排除 incompleteTrans。
                if sk.params.is_translated()
                    || sk.params.is_validated()
                    || sk.params.is_incomplete()
                {
                    return false;
                }
            }
            SstOverwriteScope::PartialOnly => {
                // 仅部分翻译项
                if !sk.params.is_incomplete() {
                    return false;
                }
            }
            SstOverwriteScope::Selection => {
                // 仅选中项
                if let Some(s_set) = selected_set {
                    if !s_set.contains(&sk.id) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }

    true
}

/// 应用字典条目，支持自定义策略
///
/// 相比 `apply_dictionary_entries`，此函数额外支持策略控制。
///
/// # 参数
/// * `strings` - 待应用翻译的 SkyString 切片
/// * `entries` - 要应用的字典条目列表
/// * `policy` - 应用策略，控制匹配行为和旧数据处理方式
///
/// # 返回
/// [`MatchResult`] 包含各层级命中数、歧义数、未匹配数等统计信息
pub fn apply_dictionary_entries_with_policy(
    strings: &mut [SkyString],
    entries: &[DictionaryApplyEntry],
    policy: ApplyPolicy,
) -> MatchResult {
    let selected_set: Option<HashSet<u32>> = policy
        .sst_options
        .as_ref()
        .and_then(|o| o.selected_ids.as_ref())
        .map(|v| v.iter().copied().collect());

    let filtered_set: Option<HashSet<u32>> = policy
        .sst_options
        .as_ref()
        .and_then(|o| o.filtered_ids.as_ref())
        .map(|v| v.iter().copied().collect());

    // Delphi 的 Apply_StringOnly 走 `findStrMatchEx(vlist, dlist, ...)`：
    // 它是逐“目标字符串”搜索词典，而不是逐“词典条目”寻找唯一目标。
    // 因此同一条 SST 源文允许应用到多个目标行，不能复用下面 entry-centric 的 4-Tier 主循环。
    if policy
        .sst_options
        .as_ref()
        .map(|opts| opts.match_mode == SstMatchMode::StringOnly)
        .unwrap_or(false)
    {
        return apply_sst_string_only_target_centric(
            strings,
            entries,
            &policy,
            selected_set.as_ref(),
            filtered_set.as_ref(),
        );
    }

    let mut result = MatchResult::default();
    let mut matched_ids: HashSet<u32> = HashSet::new();
    let index = MatchIndex::build(strings);

    // 先按原始状态冻结 SST 高级模式的候选 ID，避免应用过程中的状态变化
    // 反过来改变 scope / filter / selection 的含义。
    let eligible_ids: HashSet<u32> = policy
        .sst_options
        .as_ref()
        .map(|_| {
            strings
                .iter()
                .filter(|sk| {
                    is_candidate_eligible(sk, &policy, selected_set.as_ref(), filtered_set.as_ref())
                })
                .map(|sk| sk.id)
                .collect()
        })
        .unwrap_or_default();

    // Delphi 在显式 Reset StringState 或目标带有 nTrans 标记时，
    // 都会在匹配前重置当前覆盖范围内的目标行；未命中项也保持重置结果。
    let reset_ids: HashSet<u32> = policy
        .sst_options
        .as_ref()
        .map(|opts| {
            strings
                .iter()
                .filter(|sk| {
                    eligible_ids.contains(&sk.id)
                        && (opts.reset_state
                            || sk.internal_params.is_set(SkyStringInternalParams::N_TRANS))
                })
                .map(|sk| sk.id)
                .collect()
        })
        .unwrap_or_default();

    for entry in entries {
        match match_entry_with_policy(
            strings,
            &index,
            entry,
            &matched_ids,
            &policy,
            selected_set.as_ref(),
            filtered_set.as_ref(),
        ) {
            EntryOutcome::Matched(tier, idx) => {
                let matched_id = strings[idx].id;
                let effect = apply_match(
                    strings,
                    idx,
                    entry,
                    tier,
                    &policy,
                    reset_ids.contains(&matched_id),
                    &mut result,
                );
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
                preserve_old_data(entry, &policy, &mut result);
            }
            EntryOutcome::Unmatched => {
                result.unmatched += 1;
                preserve_old_data(entry, &policy, &mut result);
            }
        }
    }

    if !reset_ids.is_empty() {
        for sk in strings.iter_mut() {
            let reset_vmad = is_vmad_string(sk);
            if reset_ids.contains(&sk.id)
                && !matched_ids.contains(&sk.id)
                && reset_target(sk, reset_vmad)
            {
                push_updated_id(&mut result.updated_ids, sk.id);
            }
        }
    }

    result
}

/// Delphi `findStrMatchEx` 的 SST StringOnly 专用路径。
///
/// 与通用 matcher 最大的结构差异是：这里逐目标行处理，因此同一个字典条目可以命中
/// 多个具有相同源文的目标字符串。目标侧重复不是歧义；字典侧重复才需要 REC/FIELD 消歧。
fn apply_sst_string_only_target_centric(
    strings: &mut [SkyString],
    entries: &[DictionaryApplyEntry],
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
) -> MatchResult {
    let mut result = MatchResult::default();
    let mut used_entries: HashSet<usize> = HashSet::new();
    let mut by_source_hash: HashMap<u32, Vec<usize>> = HashMap::with_capacity(entries.len());

    for (entry_idx, entry) in entries.iter().enumerate() {
        by_source_hash
            .entry(string_hash(&entry.source))
            .or_default()
            .push(entry_idx);
    }

    for target_idx in 0..strings.len() {
        // VMAD 项在 Delphi 中受 `getfProcCompareOptVMADString` 控制（由 `is_candidate_eligible` 精确判定）：
        // 在 StringOnly 模式下，All / NoTrans 自动屏蔽，仅在 PartialOnly / Selection 显式范围下允许匹配。
        if !is_candidate_eligible(&strings[target_idx], policy, selected_set, filtered_set) {
            continue;
        }

        // `findStrMatchEx` 在真正搜索前执行 Reset StringState；既有
        // nTrans 标记时也会触发一次 resetTrans。
        let reset_before_match = policy
            .sst_options
            .as_ref()
            .map(|opts| {
                opts.reset_state
                    || strings[target_idx]
                        .internal_params
                        .is_set(SkyStringInternalParams::N_TRANS)
            })
            .unwrap_or(false);
        let reset_vmad = is_vmad_string(&strings[target_idx]);
        if reset_before_match && reset_target(&mut strings[target_idx], reset_vmad) {
            push_updated_id(&mut result.updated_ids, strings[target_idx].id);
        }

        // forceAutoTranslate 在同语言模式下仍会被 Delphi 的 `not TESVTSameLanguage` 关掉。
        if policy.same_language {
            continue;
        }

        let source = strings[target_idx].source.clone();
        if source.is_empty() && strings[target_idx].translation.is_empty() {
            if !policy.tag_only {
                strings[target_idx].params = SkyStringParams::new();
                strings[target_idx]
                    .params
                    .set(SkyStringParams::TRANSLATED, true);
                push_updated_id(&mut result.updated_ids, strings[target_idx].id);
            }
            continue;
        }

        let source_hash = string_hash(&source);
        let candidates: Vec<usize> = by_source_hash
            .get(&source_hash)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&entry_idx| entries[entry_idx].source == source)
            .collect();

        let Some((entry_idx, ambiguous_source)) =
            choose_string_only_entry(&strings[target_idx], entries, &candidates)
        else {
            // Legacy StringOnly 的危险但真实语义：非同语言时，未命中的 eligible 行也 resetTrans。
            // Tag Only 在 Delphi 这里本身是坏的（仍会改文本）；现代 UI 明确承诺“只打标签”，
            // 因而 tag_only 时有意不复制这个 legacy bug。
            if !policy.tag_only {
                let reset_vmad = is_vmad_string(&strings[target_idx]);
                if reset_target(&mut strings[target_idx], reset_vmad) {
                    push_updated_id(&mut result.updated_ids, strings[target_idx].id);
                }
            }
            continue;
        };

        used_entries.insert(entry_idx);
        let effect = apply_match(
            strings,
            target_idx,
            &entries[entry_idx],
            MatchTier::Exact,
            policy,
            false,
            &mut result,
        );

        if effect == ApplyEffect::PendingSkipped {
            result.pending_skipped += 1;
            continue;
        }

        result.tier_exact += 1;
        if ambiguous_source && !policy.tag_only {
            let sk = &mut strings[target_idx];
            sk.params = SkyStringParams::new();
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
            sk.internal_params
                .set(SkyStringInternalParams::N_TRANS, true);
            push_updated_id(&mut result.updated_ids, sk.id);
            result.ambiguous += 1;
        }
    }

    // Delphi 用 SSTApplied 标记真正被消费过的来源条目；未使用来源保留为 oldData。
    for (entry_idx, entry) in entries.iter().enumerate() {
        if !used_entries.contains(&entry_idx) {
            result.unmatched += 1;
            preserve_old_data(entry, policy, &mut result);
        }
    }

    result
}

/// 模拟 `getStrWithRefRec`：字典侧同源文重复时优先 REC/FIELD 相符项；
/// 若仍有多项或没有引用相符项，Delphi 会取第一项并以 nTrans/incomplete 标记不确定性。
fn choose_string_only_entry(
    target: &SkyString,
    entries: &[DictionaryApplyEntry],
    candidates: &[usize],
) -> Option<(usize, bool)> {
    match candidates {
        [] => None,
        [only] => Some((*only, false)),
        _ => {
            let referenced: Vec<usize> = candidates
                .iter()
                .copied()
                .filter(|&entry_idx| {
                    let entry = &entries[entry_idx];
                    entry.record_sig == target.esp_ptr.record_sig
                        && entry.field_sig == target.esp_ptr.field_sig
                })
                .collect();

            match referenced.as_slice() {
                [only] => Some((*only, false)),
                [first, ..] => Some((*first, true)),
                [] => Some((candidates[0], true)),
            }
        }
    }
}

fn match_entry_with_policy(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
) -> EntryOutcome {
    if let Some(opts) = &policy.sst_options {
        // Delphi 的 doApplySst 将 VMAD 的 EDID 路径单独固定到 V4Strict；
        // 因此三个 FormID 档位都要对 VMAD 强制原文与 index 精确校验。
        let is_vmad_entry = entry.field_sig == *b"VMAD";
        match opts.match_mode {
            SstMatchMode::FormIdOnly => {
                // Delphi 的 VMAD 分支固定使用 V4Strict；普通字符串仍使用 V4Edid。
                match find_sst_form_id_match(
                    strings,
                    index,
                    entry,
                    matched_ids,
                    policy,
                    selected_set,
                    filtered_set,
                    is_vmad_entry,
                    true,
                ) {
                    TierMatch::Unique(idx) => EntryOutcome::Matched(MatchTier::Exact, idx),
                    TierMatch::Ambiguous => EntryOutcome::Ambiguous,
                    TierMatch::None => EntryOutcome::Unmatched,
                }
            }
            SstMatchMode::FormIdStrictString => {
                match find_sst_form_id_match(
                    strings,
                    index,
                    entry,
                    matched_ids,
                    policy,
                    selected_set,
                    filtered_set,
                    true,
                    true,
                ) {
                    TierMatch::Unique(idx) => EntryOutcome::Matched(MatchTier::Exact, idx),
                    TierMatch::Ambiguous => EntryOutcome::Ambiguous,
                    TierMatch::None => EntryOutcome::Unmatched,
                }
            }
            SstMatchMode::FormIdRelaxedString => {
                // Delphi 的 VMAD 分支固定使用 V4Strict；普通字符串使用 V4Relax。
                match find_sst_form_id_match(
                    strings,
                    index,
                    entry,
                    matched_ids,
                    policy,
                    selected_set,
                    filtered_set,
                    true,
                    is_vmad_entry,
                ) {
                    TierMatch::Unique(idx) => EntryOutcome::Matched(MatchTier::Exact, idx),
                    TierMatch::Ambiguous => EntryOutcome::Ambiguous,
                    TierMatch::None => EntryOutcome::Unmatched,
                }
            }
            SstMatchMode::StringOnly => {
                // Delphi findStrMatchEx 强制 AutoTranslate；同语言模式下 AutoTranslate=false，
                // 因而 StringOnly 不会产生自动匹配。
                if policy.same_language {
                    return EntryOutcome::Unmatched;
                }
                match find_sst_string_only(
                    strings,
                    index,
                    entry,
                    matched_ids,
                    policy,
                    selected_set,
                    filtered_set,
                ) {
                    TierMatch::Unique(idx) => EntryOutcome::Matched(MatchTier::Exact, idx),
                    TierMatch::Ambiguous => EntryOutcome::Ambiguous,
                    TierMatch::None => EntryOutcome::Unmatched,
                }
            }
        }
    } else {
        // 标准 4-Tier 匹配流程 (T1 -> T2 -> T3 -> T4)
        match find_tier1_filtered(
            strings,
            index,
            entry,
            matched_ids,
            policy,
            selected_set,
            filtered_set,
        ) {
            TierMatch::Unique(idx) => return EntryOutcome::Matched(MatchTier::Exact, idx),
            TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
            TierMatch::None => {}
        }
        match find_tier2_filtered(
            strings,
            index,
            entry,
            matched_ids,
            policy,
            selected_set,
            filtered_set,
        ) {
            TierMatch::Unique(idx) => return EntryOutcome::Matched(MatchTier::Edid, idx),
            TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
            TierMatch::None => {}
        }
        match find_tier3_filtered(
            strings,
            index,
            entry,
            matched_ids,
            policy,
            selected_set,
            filtered_set,
        ) {
            TierMatch::Unique(idx) => return EntryOutcome::Matched(MatchTier::Normalized, idx),
            TierMatch::Ambiguous => return EntryOutcome::Ambiguous,
            TierMatch::None => {}
        }
        match find_tier4_filtered(
            strings,
            index,
            entry,
            matched_ids,
            policy,
            selected_set,
            filtered_set,
        ) {
            TierMatch::Unique(idx) => EntryOutcome::Matched(MatchTier::Vocab, idx),
            TierMatch::Ambiguous => EntryOutcome::Ambiguous,
            TierMatch::None => EntryOutcome::Unmatched,
        }
    }
}

/// 转换 XML 条目并通过共享匹配器应用它们。
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

/// 带策略的 XML 导入入口点（processor / 高级 XML 导入使用）。
///
/// Delphi `XMLImportbase`（TESVT_XMLFunc.pas）与 `batcherImportFile` 语义：
/// - 覆盖范围 / 匹配模式与 SST 应用共用同一套 comparator 家族
///   （param1 → `getfProcCompareOpt` 五档 overwrite scope，param2 →
///   `getProcSortCompare` 的 V4Edid/V4Strict/V4Relax + fallback）；
/// - 未匹配的 XML 条目直接丢弃，不生成 OLD_DATA（与 SST 加载不同），
///   因此这里强制 `preserve_old_data = false`。
pub fn apply_xml_dictionary_entries_with_policy(
    strings: &mut [SkyString],
    xml_entries: &[XmlStringEntry],
    policy: ApplyPolicy,
) -> MatchResult {
    let mut policy = policy;
    policy.preserve_old_data = false;
    let entries: Vec<DictionaryApplyEntry> = xml_entries
        .iter()
        .map(DictionaryApplyEntry::from_xml_entry)
        .collect();
    apply_dictionary_entries_with_policy(strings, &entries, policy)
}

/// 保持向下兼容的 XML 导入入口点。
pub fn enhanced_import_match(
    strings: &mut [SkyString],
    xml_entries: &[XmlStringEntry],
) -> MatchResult {
    apply_xml_dictionary_entries(strings, xml_entries)
}

// ── 各层级查找逻辑 ──

/// Tier 1: 精确的三元组匹配（带过滤）。
fn find_tier1_filtered(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
) -> TierMatch {
    let candidates = index.exact_candidates((entry.str_id, entry.record_sig, entry.field_sig));
    let mut found = None;
    for &idx in candidates {
        let sk = &strings[idx];
        if matched_ids.contains(&sk.id)
            || !is_candidate_eligible(sk, policy, selected_set, filtered_set)
        {
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

/// Tier 1: FormID + 严格文本一致匹配。
fn find_tier2_filtered(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
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
        .filter(|&idx| {
            let sk = &strings[idx];
            !matched_ids.contains(&sk.id)
                && is_candidate_eligible(sk, policy, selected_set, filtered_set)
        })
        .collect();

    match candidates.len() {
        0 => TierMatch::None,
        1 => TierMatch::Unique(candidates[0]),
        _ => disambiguate_by_normalized(strings, &candidates, &entry.source),
    }
}

/// Delphi SST V4 的三种 FormID 模式。
///
/// 共同键：sanitize(FormID) + EDID hash + field。
/// - V4Edid: 还要求 index；不检查源文本。
/// - V4Strict: 还要求精确源文本 + index。
/// - V4Relax: 还要求精确源文本；忽略 index。
fn find_sst_form_id_match(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
    require_source: bool,
    require_index: bool,
) -> TierMatch {
    let candidates = index.form_id_candidates((
        sanitize_form_id(entry.form_id),
        entry.edid_hash.unwrap_or(0),
    ));
    let entry_is_vmad = entry.field_sig == *b"VMAD";
    let mut found = None;
    for &idx in candidates {
        let sk = &strings[idx];
        if matched_ids.contains(&sk.id)
            || !is_candidate_eligible(sk, policy, selected_set, filtered_set)
            || sk.esp_ptr.field_sig != entry.field_sig
            || is_vmad_string(sk) != entry_is_vmad
            || (require_source
                && (sk.hash != string_hash(&entry.source) || sk.source != entry.source))
            || (require_index && sk.esp_ptr.index != entry.index)
        {
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

/// Delphi Apply_StringOnly：忽略 FormID，仅按源文本精确匹配。
///
/// 当同一源文本出现多次时，优先用 REC/FIELD 引用缩小候选；仍不唯一则视为歧义，
/// 不使用 T3 规范化或 T4 Jaccard 模糊匹配。
fn find_sst_string_only(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
) -> TierMatch {
    let source_hash = string_hash(&entry.source);
    let candidates: Vec<usize> = index
        .source_candidates(source_hash)
        .iter()
        .copied()
        .filter(|&idx| {
            let sk = &strings[idx];
            !matched_ids.contains(&sk.id)
                && is_candidate_eligible(sk, policy, selected_set, filtered_set)
                && sk.source == entry.source
        })
        .collect();

    match candidates.as_slice() {
        [] => TierMatch::None,
        [idx] => TierMatch::Unique(*idx),
        _ => {
            let referenced: Vec<usize> = candidates
                .into_iter()
                .filter(|&idx| {
                    strings[idx].esp_ptr.record_sig == entry.record_sig
                        && strings[idx].esp_ptr.field_sig == entry.field_sig
                })
                .collect();
            match referenced.as_slice() {
                [idx] => TierMatch::Unique(*idx),
                _ => TierMatch::Ambiguous,
            }
        }
    }
}

/// Tier 3: 规范化源文本匹配（带过滤）。
fn find_tier3_filtered(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
) -> TierMatch {
    let norm = normalization::normalize(&entry.source);
    if norm.is_empty() {
        return TierMatch::None;
    }
    let norm_hash = string_hash(&norm);
    let candidates = index.normalized_candidates((norm_hash, entry.record_sig, entry.field_sig));
    let mut found = None;
    for &idx in candidates {
        let sk = &strings[idx];
        if matched_ids.contains(&sk.id)
            || !is_candidate_eligible(sk, policy, selected_set, filtered_set)
        {
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

/// Tier 4: 词汇重叠匹配（带过滤）。
fn find_tier4_filtered(
    strings: &[SkyString],
    index: &MatchIndex,
    entry: &DictionaryApplyEntry,
    matched_ids: &HashSet<u32>,
    policy: &ApplyPolicy,
    selected_set: Option<&HashSet<u32>>,
    filtered_set: Option<&HashSet<u32>>,
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
        if matched_ids.contains(&sk.id)
            || !is_candidate_eligible(sk, policy, selected_set, filtered_set)
        {
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
        (None, _) => TierMatch::None,
    }
}

// ── 辅助函数 ──

/// 将匹配应用到目标字符串。
fn apply_match(
    strings: &mut [SkyString],
    idx: usize,
    entry: &DictionaryApplyEntry,
    tier: MatchTier,
    policy: &ApplyPolicy,
    reset_state: bool,
    result: &mut MatchResult,
) -> ApplyEffect {
    let sk = &mut strings[idx];
    let mut changed = false;

    let target_is_vmad = is_vmad_string(sk);
    if reset_state {
        changed |= reset_target(sk, target_is_vmad);
    }

    // Delphi computes VMAD's `bCheckDiff` after any resetTrans call and before
    // replacing the target translation. Preserve that pre-apply hash for the
    // VMAD-specific status mapping below.
    let target_hash_trans_before = sk.hash_trans;

    if policy.replace_string_id && sk.esp_ptr.str_id != entry.str_id {
        sk.esp_ptr.str_id = entry.str_id;
        sk.internal_params
            .set(SkyStringInternalParams::STRING_ID_CHANGED, true);
        changed = true;
    }

    // Delphi doApplySst 调用 findEdidMatchEx 时 bApplytag 恒为 true；
    // Tag Only 只控制是否写文本，不控制协作标签是否同步。
    if entry.source_format == DictionarySourceFormat::Sst && sk.colab_id != entry.colab_id {
        sk.colab_id = entry.colab_id;
        changed = true;
    }

    if policy.tag_only {
        if changed {
            push_updated_id(&mut result.updated_ids, sk.id);
        }
        return ApplyEffect::Applied;
    }

    if entry.params.map(|p| p.is_pending()).unwrap_or(false) {
        if entry.source_format == DictionarySourceFormat::Sst {
            changed |= reset_target(sk, target_is_vmad);
        }
        if changed {
            push_updated_id(&mut result.updated_ids, sk.id);
        }
        return ApplyEffect::PendingSkipped;
    }

    if !entry.translation.is_empty() && sk.translation != entry.translation {
        sk.set_translation(entry.translation.clone());
        changed = true;
    }

    let old_params = sk.params;
    let old_internal_params = sk.internal_params;
    clear_warning_flags(&mut sk.internal_params);
    apply_status(sk, entry, policy, target_hash_trans_before);
    apply_index_warning(sk, entry, tier, result);
    changed |= sk.params != old_params || sk.internal_params != old_internal_params;

    if changed {
        push_updated_id(&mut result.updated_ids, sk.id);
    }

    ApplyEffect::Applied
}

fn preserve_old_data(entry: &DictionaryApplyEntry, policy: &ApplyPolicy, result: &mut MatchResult) {
    if policy.preserve_old_data && entry.source_format == DictionarySourceFormat::Sst {
        result.old_data_preserved += 1;
        result.old_data_entries.push(entry.clone());
    }
}

fn apply_status(
    sk: &mut SkyString,
    entry: &DictionaryApplyEntry,
    policy: &ApplyPolicy,
    target_hash_trans_before: u32,
) {
    // Delphi resetStatus(v1) 会用 v1 整体替换 sparams，而不是只清四个翻译状态位。
    sk.params = SkyStringParams::new();
    let params = entry.params.unwrap_or_default();
    let is_sst_string_only = entry.source_format == DictionarySourceFormat::Sst
        && policy
            .sst_options
            .as_ref()
            .map(|opts| opts.match_mode == SstMatchMode::StringOnly)
            .unwrap_or(false);
    let is_sst_vmad = entry.source_format == DictionarySourceFormat::Sst && is_vmad_string(sk);

    if params.is_locked() {
        sk.params.set(SkyStringParams::LOCKED_TRANS, true);
    } else if params.is_incomplete() {
        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
    } else if is_sst_string_only {
        // doApplySst -> findStrMatchEx 固定传入 validatedTrans=[validated]。
        sk.params.set(SkyStringParams::VALIDATED, true);
    } else if is_sst_vmad {
        // The dedicated Delphi VMAD apply path uses appliedTrans=[validated]
        // and validatedTrans=[translated]. A changed target therefore remains
        // validated for review; an identical translation is fully translated.
        if target_hash_trans_before != string_hash(&entry.translation)
            || (policy.same_language && sk.hash != sk.hash_trans)
        {
            sk.params.set(SkyStringParams::VALIDATED, true);
        } else {
            sk.params.set(SkyStringParams::TRANSLATED, true);
        }
    } else if policy.same_language {
        sk.params.set(SkyStringParams::VALIDATED, true);
    } else if !sk.translation.is_empty() {
        sk.params.set(SkyStringParams::TRANSLATED, true);
    } else {
        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
    }
}

/// 按 Delphi `resetTrans(bResetVmad)` 重置目标字符串。
///
/// 普通字符串以空译文表示回退到源文；VMAD 没有独立的空译文表示，专用 reset
/// 会把译文设回源文，并按 Delphi `resetTrans` 的 `resetStatus([lockedTrans])`
/// 语义设置 `lockedTrans`，避免再次被当作普通未翻译项消费。
fn reset_target(sk: &mut SkyString, reset_vmad: bool) -> bool {
    let old_translation = sk.translation.clone();
    let old_params = sk.params;
    let old_internal_params = sk.internal_params;
    let old_ld_result = sk.ld_result;
    let old_ld_found = sk.ld_found;
    let is_vmad_reset = reset_vmad && is_vmad_string(sk);
    let was_locked_vmad = is_vmad_reset && sk.params.is_locked();

    let reset_translation = if is_vmad_reset {
        sk.source.clone()
    } else {
        String::new()
    };
    if sk.translation != reset_translation {
        sk.set_translation(reset_translation);
    }

    if !was_locked_vmad {
        sk.params = SkyStringParams::new();
        if is_vmad_reset {
            // Delphi resetTrans 对 VMAD 总是 resetStatus([lockedTrans])，即使
            // reset 前目标尚未 lockedTrans 也会在此处落到 lockedTrans。
            sk.params.set(SkyStringParams::LOCKED_TRANS, true);
        }
        clear_warning_flags(&mut sk.internal_params);
        sk.ld_result = 99.0;
        sk.ld_found = 0;
    }

    sk.translation != old_translation
        || sk.params != old_params
        || sk.internal_params != old_internal_params
        || sk.ld_result != old_ld_result
        || sk.ld_found != old_ld_found
}

fn push_updated_id(updated_ids: &mut Vec<u32>, id: u32) {
    if !updated_ids.contains(&id) {
        updated_ids.push(id);
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
    params.set(SkyStringInternalParams::SPELL_CHECK_FAULT, false);
}

/// 使用规范化后的源文本来消除 EDID 候选的歧义。
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
        // strings 集合: {Retrieve, the, ancient, sword, from, tomb} = 6
        // entry 集合:   {Retrieve, the, sword} = 3
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

        // Tier 3 规范化匹配应在词汇重叠匹配之前胜出。
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
    fn test_pending_sst_entry_resets_target_like_delphi() {
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
        assert!(strings[0].translation.is_empty());
        assert!(!strings[0].params.is_translated());
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

    fn make_dict_entry(
        str_id: i32,
        edid: Option<&str>,
        record_sig: [u8; 4],
        field_sig: [u8; 4],
        source: &str,
        translation: &str,
    ) -> DictionaryApplyEntry {
        DictionaryApplyEntry {
            source_format: DictionarySourceFormat::Sst,
            list_index: 0,
            str_id,
            form_id: 0,
            record_sig,
            field_sig,
            edid: edid.map(String::from),
            edid_hash: edid.map(string_hash),
            source: source.to_string(),
            translation: translation.to_string(),
            params: None,
            colab_id: 0,
            index: 0,
            index_max: 0,
        }
    }

    fn make_xml_entry(
        str_id: i32,
        edid: Option<&str>,
        record_sig: [u8; 4],
        field_sig: [u8; 4],
        source: &str,
        translation: &str,
    ) -> crate::xml::XmlStringEntry {
        crate::xml::XmlStringEntry {
            list_index: 0,
            str_id,
            edid: edid.map(String::from),
            record_sig,
            field_sig,
            index: 0,
            index_max: 0,
            source: source.to_string(),
            translation: translation.to_string(),
        }
    }

    fn xml_policy(scope: SstOverwriteScope, mode: SstMatchMode) -> ApplyPolicy {
        let mut p = ApplyPolicy::sst_load_with_options(SstApplyOptions {
            overwrite_scope: scope,
            match_mode: mode,
            ..Default::default()
        });
        p.preserve_old_data = false;
        p
    }

    #[test]
    fn test_xml_import_overwrite_scope_matrix() {
        // R-04：Delphi batcherImportFile 的 ImportXml 与 ApplySst 共用
        // param1（五档 overwrite scope）与 param2（四档 match mode）。
        // 同一输入在不同 scope 下必须产生可预测差异。
        fn build_strings() -> Vec<SkyString> {
            let mut sk1 = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
            sk1.params.set(SkyStringParams::TRANSLATED, true);
            sk1.translation = "旧苹果".to_string();
            let mut sk2 = make_sk(1, "Banana", 11, *b"INGR", *b"FULL", 0);
            sk2.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
            sk2.translation = "香蕉部分".to_string();
            let sk3 = make_sk(2, "Cherry", 12, *b"INGR", *b"FULL", 0);
            vec![sk1, sk2, sk3]
        }
        let entries = vec![
            make_xml_entry(10, None, *b"INGR", *b"FULL", "Apple", "新苹果"),
            make_xml_entry(11, None, *b"INGR", *b"FULL", "Banana", "新香蕉"),
            make_xml_entry(12, None, *b"INGR", *b"FULL", "Cherry", "潔桃"),
        ];

        // All：覆盖全部（含已翻译/部分翻译）
        let mut strings = build_strings();
        let result = apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(SstOverwriteScope::All, SstMatchMode::FormIdStrictString),
        );
        assert_eq!(result.total_matched(), 3);
        assert_eq!(strings[0].translation, "新苹果");
        assert_eq!(strings[1].translation, "新香蕉");
        assert_eq!(strings[2].translation, "潔桃");

        // NoTransExclusive：已翻译项被排除，未翻译/部分翻译可命中
        let mut strings = build_strings();
        let result = apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(
                SstOverwriteScope::NoTransExclusive,
                SstMatchMode::FormIdStrictString,
            ),
        );
        assert_eq!(strings[0].translation, "旧苹果");
        assert_eq!(strings[1].translation, "新香蕉");
        assert_eq!(strings[2].translation, "潔桃");
        assert!(result.total_matched() >= 2);

        // NoTransAndPartial：incomplete 也被排除
        let mut strings = build_strings();
        apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(
                SstOverwriteScope::NoTransAndPartial,
                SstMatchMode::FormIdStrictString,
            ),
        );
        assert_eq!(strings[0].translation, "旧苹果");
        assert_eq!(strings[1].translation, "香蕉部分");
        assert_eq!(strings[2].translation, "潔桃");

        // PartialOnly：仅 incomplete 项可命中
        let mut strings = build_strings();
        apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(
                SstOverwriteScope::PartialOnly,
                SstMatchMode::FormIdStrictString,
            ),
        );
        assert_eq!(strings[0].translation, "旧苹果");
        assert_eq!(strings[1].translation, "新香蕉");
        assert_eq!(strings[2].translation, "");

        // Selection：无 selected_ids 时 fail-closed，全部不命中
        let mut strings = build_strings();
        let result = apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(
                SstOverwriteScope::Selection,
                SstMatchMode::FormIdStrictString,
            ),
        );
        assert_eq!(result.total_matched(), 0);
        assert_eq!(strings[0].translation, "旧苹果");

        // Selection + selected_ids：仅选中项命中
        let mut strings = build_strings();
        let policy = {
            let mut p = xml_policy(
                SstOverwriteScope::Selection,
                SstMatchMode::FormIdStrictString,
            );
            if let Some(opts) = p.sst_options.as_mut() {
                opts.selected_ids = Some(vec![2]);
            }
            p
        };
        apply_xml_dictionary_entries_with_policy(&mut strings, &entries, policy);
        assert_eq!(strings[0].translation, "旧苹果");
        assert_eq!(strings[2].translation, "潔桃");
    }

    #[test]
    fn test_xml_import_match_mode_matrix() {
        // R-04：param2 四档 match mode 在 XML 导入下的差异。
        // XML 条目的 form_id 恒为 0（无法从 XML 获取），FormID 模式依赖
        // str_id/EDID/REC/FIELD 元数据；StringOnly 忽略 FormID 仅按原文匹配。
        fn build_strings() -> Vec<SkyString> {
            let mut sk1 = make_sk(0, "Iron Sword", 100, *b"WEAP", *b"FULL", 0);
            sk1.esp_ptr.form_id = 0x0100_0042;
            let sk2 = make_sk(1, "Iron Sword", 200, *b"WEAP", *b"FULL", 0);
            let sk3 = make_sk(2, "Silver Sword", 300, *b"WEAP", *b"FULL", 0);
            vec![sk1, sk2, sk3]
        }
        // 同一 XML 源文对应两个不同 str_id 的目标行
        let entries = vec![make_xml_entry(
            100,
            None,
            *b"WEAP",
            *b"FULL",
            "Iron Sword",
            "铁剑",
        )];

        // FormIdStrictString：XML 条目 form_id=0，候选为 form_id=0 的目标行；
        // 源文精确校验后仅命中同源文的 sk2，sk1（真实 FormID）不受影响。
        let mut strings = build_strings();
        let result = apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(SstOverwriteScope::All, SstMatchMode::FormIdStrictString),
        );
        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "");
        assert_eq!(strings[1].translation, "铁剑");

        // FormIdOnly：无源文校验时 sk2/sk3 同键候选 → 歧义不应用。
        let mut strings = build_strings();
        let result = apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(SstOverwriteScope::All, SstMatchMode::FormIdOnly),
        );
        assert_eq!(result.total_matched(), 0);
        assert_eq!(result.ambiguous, 1);

        // StringOnly：忽略 FormID，同源文的目标行全部命中
        // （Delphi findStrMatchEx 是 target-centric，同一源文允许多目标）
        let mut strings = build_strings();
        let result = apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(SstOverwriteScope::All, SstMatchMode::StringOnly),
        );
        assert_eq!(result.total_matched(), 2);
        assert_eq!(strings[0].translation, "铁剑");
        assert_eq!(strings[1].translation, "铁剑");
        assert_eq!(strings[2].translation, "");
    }

    #[test]
    fn test_xml_import_no_old_data_preserved() {
        // Delphi XMLImportbase 未匹配条目直接丢弃，不生成 OLD_DATA。
        let strings_seed = vec![make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0)];
        let entries = vec![make_xml_entry(
            999,
            None,
            *b"INGR",
            *b"FULL",
            "不存在的条目",
            "不应保留",
        )];
        let mut strings = strings_seed.clone();
        let result = apply_xml_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            xml_policy(SstOverwriteScope::All, SstMatchMode::FormIdStrictString),
        );
        assert_eq!(result.unmatched, 1);
        assert_eq!(result.old_data_preserved, 0);
        assert!(result.old_data_entries.is_empty());
        assert_eq!(strings.len(), 1); // 未追加 OLD_DATA 行
    }

    #[test]
    fn test_sst_options_overwrite_scope_notrans_exclusive() {
        let mut sk1 = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        sk1.params.set(SkyStringParams::TRANSLATED, true);
        sk1.translation = "旧苹果".to_string();

        let sk2 = make_sk(1, "Banana", 11, *b"INGR", *b"FULL", 0);
        // sk2 未翻译

        let mut strings = vec![sk1, sk2];

        let entries = vec![
            make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "新苹果"),
            make_dict_entry(11, None, *b"INGR", *b"FULL", "Banana", "香蕉"),
        ];

        let opts = SstApplyOptions {
            overwrite_scope: SstOverwriteScope::NoTransExclusive,
            match_mode: SstMatchMode::FormIdStrictString,
            ..Default::default()
        };
        let policy = ApplyPolicy::sst_load_with_options(opts);

        let result = apply_dictionary_entries_with_policy(&mut strings, &entries, policy);
        assert_eq!(result.tier_exact, 1);
        assert_eq!(strings[0].translation, "旧苹果"); // 已翻译项被跳过
        assert_eq!(strings[1].translation, "香蕉"); // 未翻译项成功覆盖
    }

    #[test]
    fn test_sst_options_overwrite_scope_partial_only() {
        let mut sk1 = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        sk1.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
        sk1.translation = "部分苹果".to_string();

        let sk2 = make_sk(1, "Banana", 11, *b"INGR", *b"FULL", 0);
        // sk2 未翻译（非 partial）

        let mut strings = vec![sk1, sk2];

        let entries = vec![
            make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "完全苹果"),
            make_dict_entry(11, None, *b"INGR", *b"FULL", "Banana", "香蕉"),
        ];

        let opts = SstApplyOptions {
            overwrite_scope: SstOverwriteScope::PartialOnly,
            match_mode: SstMatchMode::FormIdStrictString,
            ..Default::default()
        };
        let policy = ApplyPolicy::sst_load_with_options(opts);

        let result = apply_dictionary_entries_with_policy(&mut strings, &entries, policy);
        assert_eq!(result.tier_exact, 1);
        assert_eq!(strings[0].translation, "完全苹果"); // 仅 partial 被覆盖
        assert_eq!(strings[1].translation, ""); // 未翻译项被跳过
    }

    #[test]
    fn test_sst_options_overwrite_scope_selection() {
        let sk1 = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        let sk2 = make_sk(1, "Banana", 11, *b"INGR", *b"FULL", 0);

        let mut strings = vec![sk1, sk2];

        let entries = vec![
            make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "苹果"),
            make_dict_entry(11, None, *b"INGR", *b"FULL", "Banana", "香蕉"),
        ];

        let opts = SstApplyOptions {
            overwrite_scope: SstOverwriteScope::Selection,
            match_mode: SstMatchMode::FormIdStrictString,
            selected_ids: Some(vec![1]), // 仅选中 Banana (id=1)
            ..Default::default()
        };
        let policy = ApplyPolicy::sst_load_with_options(opts);

        let result = apply_dictionary_entries_with_policy(&mut strings, &entries, policy);
        assert_eq!(result.tier_exact, 1);
        assert_eq!(strings[0].translation, "");
        assert_eq!(strings[1].translation, "香蕉");
    }

    #[test]
    fn test_sst_options_match_mode_string_only() {
        // StringOnly 忽略 FormID；FormID 模式使用真实 form_id，而不是 str_id。
        let mut sk1 = make_sk(0, "Iron Sword", 999, *b"WEAP", *b"FULL", 0);
        sk1.esp_ptr.form_id = 0x0100_0042;
        let mut strings = vec![sk1];

        let mut entry = make_dict_entry(10, None, *b"WEAP", *b"FULL", "Iron Sword", "铁剑");
        entry.form_id = 0x0200_0043;
        let entries = vec![entry];

        // FormIdStrictString 因真实 FormID 不同而无法命中。
        let opts_formid = SstApplyOptions {
            match_mode: SstMatchMode::FormIdStrictString,
            ..Default::default()
        };
        let result_formid = apply_dictionary_entries_with_policy(
            &mut strings.clone(),
            &entries,
            ApplyPolicy::sst_load_with_options(opts_formid),
        );
        assert_eq!(result_formid.total_matched(), 0);

        // StringOnly 成功命中
        let opts_str = SstApplyOptions {
            match_mode: SstMatchMode::StringOnly,
            ..Default::default()
        };
        let result_str = apply_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            ApplyPolicy::sst_load_with_options(opts_str),
        );
        assert_eq!(result_str.total_matched(), 1);
        assert_eq!(strings[0].translation, "铁剑");
        assert!(strings[0].params.is_validated());
        assert!(!strings[0].params.is_translated());

        // Delphi StringOnly 不做规范化/T4 模糊匹配。
        let fuzzy_entry = make_dict_entry(11, None, *b"WEAP", *b"FULL", "iron  sword!", "不应命中");
        let fuzzy_result = apply_dictionary_entries_with_policy(
            &mut vec![make_sk(2, "Iron Sword", 123, *b"WEAP", *b"FULL", 0)],
            &[fuzzy_entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::StringOnly,
                ..Default::default()
            }),
        );
        assert_eq!(fuzzy_result.total_matched(), 0);
    }

    #[test]
    fn test_sst_string_only_resets_unmatched_eligible_targets_like_delphi() {
        let mut matched = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        matched.translation = "旧苹果".to_string();
        matched.params.set(SkyStringParams::TRANSLATED, true);

        let mut unmatched = make_sk(1, "Pear", 11, *b"INGR", *b"FULL", 0);
        unmatched.translation = "旧梨".to_string();
        unmatched.params.set(SkyStringParams::TRANSLATED, true);

        let entry = make_dict_entry(99, None, *b"MISC", *b"FULL", "Apple", "苹果");
        let mut strings = vec![matched, unmatched];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::StringOnly,
                reset_state: false,
                ..Default::default()
            }),
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "苹果");
        assert!(strings[0].params.is_validated());
        assert!(strings[1].translation.is_empty());
        assert!(!strings[1].params.is_translated());
        assert!(result.updated_ids.contains(&1));
    }

    #[test]
    fn test_sst_string_only_one_dictionary_entry_can_apply_to_multiple_targets() {
        let first = make_sk(0, "Yes.", 10, *b"INFO", *b"NAM1", 0);
        let second = make_sk(1, "Yes.", 11, *b"INFO", *b"RNAM", 0);
        let entry = make_dict_entry(99, None, *b"INFO", *b"NAM1", "Yes.", "是。 ");

        let mut strings = vec![first, second];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::StringOnly,
                ..Default::default()
            }),
        );

        assert_eq!(result.total_matched(), 2);
        assert_eq!(strings[0].translation, "是。 ");
        assert_eq!(strings[1].translation, "是。 ");
        assert_eq!(result.old_data_preserved, 0);
    }

    #[test]
    fn test_sst_string_only_tag_only_keeps_modern_only_tag_contract() {
        let mut matched = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        matched.translation = "旧苹果".to_string();
        matched.params.set(SkyStringParams::TRANSLATED, true);

        let mut unmatched = make_sk(1, "Pear", 11, *b"INGR", *b"FULL", 0);
        unmatched.translation = "旧梨".to_string();
        unmatched.params.set(SkyStringParams::TRANSLATED, true);

        let mut entry = make_dict_entry(99, None, *b"MISC", *b"FULL", "Apple", "苹果");
        entry.colab_id = 23;
        let mut strings = vec![matched, unmatched];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::StringOnly,
                tag_only: true,
                ..Default::default()
            }),
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "旧苹果");
        assert_eq!(strings[0].colab_id, 23);
        assert_eq!(strings[1].translation, "旧梨");
    }

    #[test]
    fn test_sst_string_only_does_not_autotranslate_same_language() {
        let mut strings = vec![make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0)];
        let entry = make_dict_entry(99, None, *b"MISC", *b"FULL", "Apple", "苹果");
        let mut policy = ApplyPolicy::sst_load_with_options(SstApplyOptions {
            match_mode: SstMatchMode::StringOnly,
            ..Default::default()
        });
        policy.same_language = true;

        let result = apply_dictionary_entries_with_policy(&mut strings, &[entry], policy);

        assert_eq!(result.total_matched(), 0);
        assert!(strings[0].translation.is_empty());
    }

    #[test]
    fn test_sst_options_tag_only_and_reset_state() {
        let sk1 = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        let mut strings = vec![sk1];

        let mut entry = make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "苹果");
        entry.colab_id = 42;

        // Tag Only
        let opts_tag = SstApplyOptions {
            tag_only: true,
            ..Default::default()
        };
        let result_tag = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry.clone()],
            ApplyPolicy::sst_load_with_options(opts_tag),
        );
        assert_eq!(result_tag.total_matched(), 1);
        assert_eq!(strings[0].translation, ""); // 文本未变
        assert_eq!(strings[0].colab_id, 42); // 标签已打上

        // Reset State 不会把 SST 的 incomplete 强行升级为 translated。
        let mut entry_partial = make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "苹果");
        let mut p = SkyStringParams::default();
        p.set(SkyStringParams::INCOMPLETE_TRANS, true);
        entry_partial.params = Some(p);

        let opts_reset = SstApplyOptions {
            reset_state: true,
            ..Default::default()
        };
        apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry_partial],
            ApplyPolicy::sst_load_with_options(opts_reset),
        );
        assert_eq!(strings[0].translation, "苹果");
        assert!(!strings[0].params.is_translated());
        assert!(strings[0].params.is_incomplete());
    }

    #[test]
    fn test_sst_reset_state_also_resets_unmatched_eligible_targets() {
        let mut matched = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        matched.translation = "旧苹果".to_string();
        matched.params.set(SkyStringParams::TRANSLATED, true);

        let mut unmatched = make_sk(1, "Pear", 11, *b"INGR", *b"FULL", 0);
        unmatched.translation = "旧梨".to_string();
        unmatched.params.set(SkyStringParams::TRANSLATED, true);

        let entry = make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "苹果");
        let mut strings = vec![matched, unmatched];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                reset_state: true,
                ..Default::default()
            }),
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "苹果");
        assert!(strings[0].params.is_translated());
        assert!(strings[1].translation.is_empty());
        assert!(!strings[1].params.is_translated());
        assert!(result.updated_ids.contains(&1));
    }

    #[test]
    fn test_sst_normal_apply_also_syncs_colab_tag() {
        let mut strings = vec![make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0)];
        let mut entry = make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "苹果");
        entry.colab_id = 17;

        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions::default()),
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "苹果");
        assert_eq!(strings[0].colab_id, 17);
    }

    #[test]
    fn test_sst_form_id_modes_use_real_form_id_and_delphi_index_rules() {
        let mut target = make_sk(0, "Iron Sword", 999, *b"WEAP", *b"FULL", 0xAABB_CCDD);
        target.esp_ptr.form_id = 0x0500_1234;
        target.esp_ptr.index = 2;

        let mut entry = make_dict_entry(10, None, *b"WEAP", *b"FULL", "Iron Sword", "铁剑");
        entry.form_id = 0x0900_1234; // sanitize 后与 target 相同
        entry.edid_hash = Some(0xAABB_CCDD);
        entry.index = 3;

        // FormIdOnly 与 Strict 都要求 index，因此不命中；str_id 的差异不参与 V4 FormID 键。
        for mode in [SstMatchMode::FormIdOnly, SstMatchMode::FormIdStrictString] {
            let result = apply_dictionary_entries_with_policy(
                &mut vec![target.clone()],
                &[entry.clone()],
                ApplyPolicy::sst_load_with_options(SstApplyOptions {
                    match_mode: mode,
                    ..Default::default()
                }),
            );
            assert_eq!(result.total_matched(), 0);
        }

        // V4Relax 仍要求源文本精确一致，但忽略 index。
        let mut relaxed_target = target.clone();
        let relaxed_result = apply_dictionary_entries_with_policy(
            std::slice::from_mut(&mut relaxed_target),
            &[entry.clone()],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::FormIdRelaxedString,
                ..Default::default()
            }),
        );
        assert_eq!(relaxed_result.total_matched(), 1);
        assert_eq!(relaxed_target.translation, "铁剑");

        let mut non_exact = entry;
        non_exact.source = "iron sword".to_string();
        let non_exact_result = apply_dictionary_entries_with_policy(
            &mut vec![target],
            &[non_exact],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::FormIdRelaxedString,
                ..Default::default()
            }),
        );
        assert_eq!(non_exact_result.total_matched(), 0);
    }

    #[test]
    fn test_sst_no_trans_and_partial_name_excludes_incomplete_like_delphi() {
        let plain = make_sk(0, "Apple", 10, *b"INGR", *b"FULL", 0);
        let mut partial = make_sk(1, "Pear", 11, *b"INGR", *b"FULL", 0);
        partial.params.set(SkyStringParams::INCOMPLETE_TRANS, true);

        let mut strings = vec![plain, partial];
        let entries = vec![
            make_dict_entry(10, None, *b"INGR", *b"FULL", "Apple", "苹果"),
            make_dict_entry(11, None, *b"INGR", *b"FULL", "Pear", "梨"),
        ];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &entries,
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                overwrite_scope: SstOverwriteScope::NoTransAndPartial,
                ..Default::default()
            }),
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "苹果");
        assert!(strings[1].translation.is_empty());
    }

    #[test]
    fn test_sst_form_id_keeps_locked_normal_target_excluded() {
        let mut target = make_sk(0, "Normal text", 10, *b"QUST", *b"FULL", 0x1122_3344);
        target.esp_ptr.form_id = 0x0100_0042;
        target.esp_ptr.index = 3;
        target.set_translation("旧普通译文".to_string());
        target.params.set(SkyStringParams::LOCKED_TRANS, true);

        let mut entry = make_dict_entry(99, None, *b"QUST", *b"FULL", "Normal text", "普通新译文");
        entry.form_id = 0x0100_0042;
        entry.edid_hash = Some(0x1122_3344);
        entry.index = 3;

        let mut strings = vec![target];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions::default()),
        );

        assert_eq!(result.total_matched(), 0);
        assert_eq!(strings[0].translation, "旧普通译文");
        assert!(strings[0].params.is_locked());
    }

    #[test]
    fn test_sst_vmad_form_id_modes_are_fixed_to_strict_route() {
        let edid_hash = string_hash("QuestScript\0DisplayName");
        let make_target = |source: &str, index: u16, locked: bool| {
            let mut target = make_sk(0, source, -32, *b"QUST", *b"VMAD", edid_hash);
            target.esp_ptr.form_id = 0x0100_0042;
            target.esp_ptr.index = index;
            target
                .internal_params
                .set(SkyStringInternalParams::IS_VMAD_STRING, true);
            if locked {
                target.set_translation("旧脚本文本".to_string());
                target.params.set(SkyStringParams::LOCKED_TRANS, true);
            }
            target
        };
        let make_entry = |source: &str, index: u16| {
            let mut entry = make_dict_entry(-32, None, *b"QUST", *b"VMAD", source, "任务脚本文本");
            entry.form_id = 0x0100_0042;
            entry.edid_hash = Some(edid_hash);
            entry.index = index;
            entry
        };

        // 三个用户可选的 FormID 档位都必须让 VMAD 走 V4Strict；即使目标为
        // lockedTrans，专用 compareOptVMAD 仍允许它参与匹配。
        for mode in [
            SstMatchMode::FormIdOnly,
            SstMatchMode::FormIdStrictString,
            SstMatchMode::FormIdRelaxedString,
        ] {
            let mut strings = vec![make_target("Current text", 3, true)];
            let result = apply_dictionary_entries_with_policy(
                &mut strings,
                &[make_entry("Current text", 3)],
                ApplyPolicy::sst_load_with_options(SstApplyOptions {
                    match_mode: mode,
                    ..Default::default()
                }),
            );
            assert_eq!(result.total_matched(), 1, "VMAD mode {mode:?} must match");
            assert_eq!(strings[0].translation, "任务脚本文本");
        }

        // V4Edid 不能放宽 VMAD 的源文校验。
        let source_drift_result = apply_dictionary_entries_with_policy(
            &mut vec![make_target("Current text", 3, false)],
            &[make_entry("Old text", 3)],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::FormIdOnly,
                ..Default::default()
            }),
        );
        assert_eq!(source_drift_result.total_matched(), 0);

        // V4Relax 也不能放宽 VMAD 的 index 校验。
        let index_drift_result = apply_dictionary_entries_with_policy(
            &mut vec![make_target("Current text", 3, false)],
            &[make_entry("Current text", 4)],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::FormIdRelaxedString,
                ..Default::default()
            }),
        );
        assert_eq!(index_drift_result.total_matched(), 0);
    }

    #[test]
    fn test_sst_vmad_form_id_apply_uses_review_status_for_changed_translation() {
        let edid_hash = string_hash("QuestScript\0DisplayName");
        let mut target = make_sk(0, "Quest script text", -32, *b"QUST", *b"VMAD", edid_hash);
        target.esp_ptr.form_id = 0x0100_0042;
        target.esp_ptr.index = 3;
        target
            .internal_params
            .set(SkyStringInternalParams::IS_VMAD_STRING, true);

        let mut entry = make_dict_entry(
            -32,
            None,
            *b"QUST",
            *b"VMAD",
            "Quest script text",
            "任务脚本文本",
        );
        entry.form_id = 0x0100_0042;
        entry.edid_hash = Some(edid_hash);
        entry.index = 3;

        let mut strings = vec![target];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::FormIdStrictString,
                ..Default::default()
            }),
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "任务脚本文本");
        assert!(strings[0].params.is_validated());
        assert!(!strings[0].params.is_translated());
    }

    #[test]
    fn test_sst_vmad_form_id_apply_marks_identical_translation_translated() {
        let edid_hash = string_hash("QuestScript\0DisplayName");
        let mut target = make_sk(0, "Quest script text", -32, *b"QUST", *b"VMAD", edid_hash);
        target.esp_ptr.form_id = 0x0100_0042;
        target.esp_ptr.index = 3;
        target
            .internal_params
            .set(SkyStringInternalParams::IS_VMAD_STRING, true);
        target.set_translation("任务脚本文本".to_string());
        target.params.set(SkyStringParams::TRANSLATED, true);

        let mut entry = make_dict_entry(
            -32,
            None,
            *b"QUST",
            *b"VMAD",
            "Quest script text",
            "任务脚本文本",
        );
        entry.form_id = 0x0100_0042;
        entry.edid_hash = Some(edid_hash);
        entry.index = 3;

        let mut strings = vec![target];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                match_mode: SstMatchMode::FormIdStrictString,
                ..Default::default()
            }),
        );

        assert_eq!(result.total_matched(), 1);
        assert!(strings[0].params.is_translated());
        assert!(!strings[0].params.is_validated());
    }

    #[test]
    fn test_sst_vmad_same_language_keeps_review_status_when_source_differs() {
        let edid_hash = string_hash("QuestScript\0DisplayName");
        let mut target = make_sk(0, "Quest script text", -32, *b"QUST", *b"VMAD", edid_hash);
        target.esp_ptr.form_id = 0x0100_0042;
        target.esp_ptr.index = 3;
        target
            .internal_params
            .set(SkyStringInternalParams::IS_VMAD_STRING, true);
        target.set_translation("任务脚本文本".to_string());
        target.params.set(SkyStringParams::TRANSLATED, true);

        let mut entry = make_dict_entry(
            -32,
            None,
            *b"QUST",
            *b"VMAD",
            "Quest script text",
            "任务脚本文本",
        );
        entry.form_id = 0x0100_0042;
        entry.edid_hash = Some(edid_hash);
        entry.index = 3;

        let mut strings = vec![target];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy {
                same_language: true,
                sst_options: Some(SstApplyOptions::default()),
                ..Default::default()
            },
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "任务脚本文本");
        assert!(strings[0].params.is_validated());
        assert!(!strings[0].params.is_translated());
    }

    #[test]
    fn test_sst_vmad_reset_replays_for_existing_n_trans_marker() {
        let edid_hash = string_hash("QuestScript\0DisplayName");
        let mut target = make_sk(0, "Current VMAD text", -32, *b"QUST", *b"VMAD", edid_hash);
        target.esp_ptr.form_id = 0x0100_0042;
        target.esp_ptr.index = 3;
        target
            .internal_params
            .set(SkyStringInternalParams::IS_VMAD_STRING, true);
        target.set_translation("旧脚本文本".to_string());
        target.params.set(SkyStringParams::TRANSLATED, true);
        target
            .internal_params
            .set(SkyStringInternalParams::N_TRANS, true);

        let mut entry = make_dict_entry(
            -32,
            None,
            *b"QUST",
            *b"VMAD",
            "Old VMAD text",
            "旧脚本新译文",
        );
        entry.form_id = 0x0100_0042;
        entry.edid_hash = Some(edid_hash);
        entry.index = 3;

        let mut strings = vec![target];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions::default()),
        );

        assert_eq!(result.total_matched(), 0);
        assert_eq!(strings[0].translation, "Current VMAD text");
        assert!(strings[0].params.is_locked());
        assert!(!strings[0].params.is_translated());
        assert!(!strings[0].params.is_incomplete());
        assert!(!strings[0].params.is_validated());
        assert!(!strings[0]
            .internal_params
            .is_set(SkyStringInternalParams::N_TRANS));
        assert_eq!(strings[0].hash_trans, string_hash("Current VMAD text"));
        assert_eq!(strings[0].ld_result, 99.0);
        assert!(result.updated_ids.contains(&0));
    }

    #[test]
    fn test_sst_vmad_matched_with_n_trans_resets_then_applies() {
        let edid_hash = string_hash("QuestScript\0DisplayName");
        let mut target = make_sk(0, "Current VMAD text", -32, *b"QUST", *b"VMAD", edid_hash);
        target.esp_ptr.form_id = 0x0100_0042;
        target.esp_ptr.index = 3;
        target
            .internal_params
            .set(SkyStringInternalParams::IS_VMAD_STRING, true);
        target.set_translation("旧脚本文本".to_string());
        target.params.set(SkyStringParams::TRANSLATED, true);
        target
            .internal_params
            .set(SkyStringInternalParams::N_TRANS, true);

        let mut entry = make_dict_entry(
            -32,
            None,
            *b"QUST",
            *b"VMAD",
            "Current VMAD text",
            "新脚本文本",
        );
        entry.form_id = 0x0100_0042;
        entry.edid_hash = Some(edid_hash);
        entry.index = 3;

        let mut strings = vec![target];
        let result = apply_dictionary_entries_with_policy(
            &mut strings,
            &[entry],
            ApplyPolicy::sst_load_with_options(SstApplyOptions::default()),
        );

        assert_eq!(result.total_matched(), 1);
        assert_eq!(strings[0].translation, "新脚本文本");
        assert!(!strings[0]
            .internal_params
            .is_set(SkyStringInternalParams::N_TRANS));
        assert!(strings[0].params.is_validated());
        assert!(!strings[0].params.is_locked());
    }

    #[test]
    fn test_sst_vmad_protection_in_string_only_mode() {
        let mut vmad_item = make_sk(0, "QuestScriptVar", 10, *b"VMAD", *b"EDID", 0);
        vmad_item
            .internal_params
            .set(SkyStringInternalParams::IS_VMAD_STRING, true);

        let entry = make_dict_entry(
            99,
            None,
            *b"VMAD",
            *b"EDID",
            "QuestScriptVar",
            "任务脚本变量",
        );

        // 1. StringOnly + All: VMAD 项被 Delphi compareOptBlock 保护屏蔽，不应被覆盖
        let mut strings_all = vec![vmad_item.clone()];
        let res_all = apply_dictionary_entries_with_policy(
            &mut strings_all,
            &[entry.clone()],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                overwrite_scope: SstOverwriteScope::All,
                match_mode: SstMatchMode::StringOnly,
                ..Default::default()
            }),
        );
        assert_eq!(res_all.total_matched(), 0);
        assert!(strings_all[0].translation.is_empty());

        // 2. StringOnly + NoTransExclusive: 同样被屏蔽
        let mut strings_notrans = vec![vmad_item.clone()];
        let res_notrans = apply_dictionary_entries_with_policy(
            &mut strings_notrans,
            &[entry.clone()],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                overwrite_scope: SstOverwriteScope::NoTransExclusive,
                match_mode: SstMatchMode::StringOnly,
                ..Default::default()
            }),
        );
        assert_eq!(res_notrans.total_matched(), 0);
        assert!(strings_notrans[0].translation.is_empty());

        // 3. StringOnly + NoTransAndPartial: 同样被屏蔽 (Delphi getfProcCompareOptVMADString -> compareOptBlock)
        let mut strings_notrans_partial = vec![vmad_item.clone()];
        let res_notrans_partial = apply_dictionary_entries_with_policy(
            &mut strings_notrans_partial,
            &[entry.clone()],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                overwrite_scope: SstOverwriteScope::NoTransAndPartial,
                match_mode: SstMatchMode::StringOnly,
                ..Default::default()
            }),
        );
        assert_eq!(res_notrans_partial.total_matched(), 0);
        assert!(strings_notrans_partial[0].translation.is_empty());

        // 4. StringOnly + PartialOnly: 当且仅当 VMAD 项被标记为 Partial (F2) 时，允许覆盖
        let mut vmad_partial = vmad_item.clone();
        vmad_partial
            .params
            .set(SkyStringParams::INCOMPLETE_TRANS, true);
        let mut strings_partial = vec![vmad_partial];
        let res_partial = apply_dictionary_entries_with_policy(
            &mut strings_partial,
            &[entry.clone()],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                overwrite_scope: SstOverwriteScope::PartialOnly,
                match_mode: SstMatchMode::StringOnly,
                ..Default::default()
            }),
        );
        assert_eq!(res_partial.total_matched(), 1);
        assert_eq!(strings_partial[0].translation, "任务脚本变量");

        // 5. StringOnly + Selection: 选中项也允许覆盖
        let mut strings_selection = vec![vmad_item.clone()];
        let res_selection = apply_dictionary_entries_with_policy(
            &mut strings_selection,
            &[entry.clone()],
            ApplyPolicy::sst_load_with_options(SstApplyOptions {
                overwrite_scope: SstOverwriteScope::Selection,
                match_mode: SstMatchMode::StringOnly,
                selected_ids: Some(vec![0]),
                ..Default::default()
            }),
        );
        assert_eq!(res_selection.total_matched(), 1);
        assert_eq!(strings_selection[0].translation, "任务脚本变量");
    }
}
