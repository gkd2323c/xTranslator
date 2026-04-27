use super::esp_pointer::{string_hash, EspPointer};
use super::params::{SkyStringInternalParams, SkyStringParams};
use crate::normalization;

/// SkyString - 核心字符串数据结构
///
// 对应 Delphi 的 `tSkyStr` 记录，存储单个可翻译字符串的完整信息
// 字段命名和用途与 Delphi 原版保持一致，确保 SST 字典兼容性
///
// 使用约束：
// - `id`：仅运行时稳定，不可作为跨会话持久化主键。
// - 持久化匹配应优先依赖 `esp_ptr` 三元组（str_id/record_sig/field_sig）。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SkyString {
    /// 内部唯一 ID（运行时分配，不持久化到 SST）
    pub id: u32,

    /// 源字符串（原文，对应 Delphi 的 gS 字段）
    pub source: String,
    /// 翻译字符串（译文，对应 Delphi 的 gTrans 字段）
    pub translation: String,

    /// 所属记录签名 (如 "INFO")
    pub record_sig: [u8; 4],
    /// 所属字段签名 (如 "DESC")
    pub field_sig: [u8; 4],

    /// 规范化后的源字符串（用于模糊匹配，对应 Delphi 的 gSNormalized）
    pub source_normalized: Option<String>,
    /// 规范化字符串的哈希值（对应 Delphi 的 fNormalizedHash）
    pub normalized_hash: Option<u32>,

    /// 源字符串的 FNV-1a 哈希值（用于快速比对）
    pub hash: u32,
    /// 翻译字符串的 FNV-1a 哈希值（用于检测翻译变化）
    pub hash_trans: u32,

    /// 分词哈希列表 - 启发式搜索的核心数据（对应 Delphi 的 aWords 数组）
    /// 存储源字符串分词后的各个单词哈希，用于相似度计算
    pub word_hashes: Vec<u32>,

    /// REC:FIELD 引用列表（对应 Delphi 的 aRecRef 数组）
    /// 记录该字符串被哪些 REC:FIELD 引用，用于交叉验证
    pub rec_refs: Vec<u64>,

    /// ESP 指针 - 精确定位字符串在 ESP 文件中的位置
    /// 对应 Delphi 的 esp 字段，类型为 rEspPointerLite
    pub esp_ptr: EspPointer,

    /// 父记录 FormID（运行时从 GRUP 层级提取，不持久化到 SST）
    /// 例如：INFO 记录的 parent_form_id = 所属 DIAL 的 FormID
    pub parent_form_id: u32,

    /// 状态参数（持久化到 SST 字典，对应 Delphi 的 sparams 集合）
    /// 包含 translated/locked/incomplete 等标志位
    pub params: SkyStringParams,

    /// 内部参数（仅运行时使用，不持久化，对应 Delphi 的 sInternalparams）
    /// 包含缓存标志、警告状态等临时信息
    pub internal_params: SkyStringInternalParams,

    /// Strings 文件类型索引：0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
    /// 对应 record_defs 中的 list_index，用于确定翻译写入哪个 Strings 文件
    pub list_index: u8,

    /// 协作 ID（0-255，对应 Delphi 的 colabId）
    /// 用于多人协作翻译时的记录归属
    pub colab_id: u8,

    /// 启发式搜索匹配度（对应 Delphi 的 LDResult）
    /// 值范围 0.0~1.0，表示与搜索词的相似度
    pub ld_result: f32,
    /// 低距离匹配数量（对应 Delphi 的 LDFound）
    /// 启发式搜索中找到的近似匹配项数量
    pub ld_found: i32,
    /// 词数阈值（对应 Delphi 的 minWord）
    /// 启发式搜索的最少匹配词数
    pub min_word: i32,

    /// 标签哈希值（对应 Delphi 的 iTagHash）
    /// 用于标记和分类字符串
    pub tag_hash: u32,
}

impl SkyString {
    /// 创建新的 SkyString 实例
    ///
    /// # 参数
    /// * `id` - 字符串 ID
    /// * `source` - 源文本
    /// * `translation` - 译文
    /// * `record_sig` - 所属记录签名 (如 "INFO")
    /// * `field_sig` - 所属字段签名 (如 "DESC")
    pub fn new(
        id: u32,
        source: String,
        translation: String,
        record_sig: [u8; 4],
        field_sig: [u8; 4],
    ) -> Self {
        // 计算哈希
        let hash = string_hash(&source);
        let hash_trans = string_hash(&translation);
        // 计算分词哈希列表（简单按非字母数字分割）
        let word_hashes = source
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(string_hash)
            .collect();

        // 计算源文本的规范化版本及其哈希
        let source_normalized = normalization::normalize(&source);
        let normalized_hash = if !source_normalized.is_empty() {
            Some(string_hash(&source_normalized))
        } else {
            None
        };

        Self {
            id,
            source,
            translation,
            record_sig,
            field_sig,
            hash,
            hash_trans,
            word_hashes,
            source_normalized: if source_normalized.is_empty() {
                None
            } else {
                Some(source_normalized)
            },
            normalized_hash,
            esp_ptr: EspPointer::null(),
            params: SkyStringParams::default(),
            internal_params: SkyStringInternalParams::default(),
            list_index: 0,
            colab_id: 0,
            ld_result: 0.0,
            ld_found: 0,
            min_word: 0,
            tag_hash: 0,
            rec_refs: Vec::new(),
            parent_form_id: 0,
        }
    }

    /// 更新源字符串并重新计算哈希
    pub fn set_source(&mut self, source: String) {
        // hash 与文本必须保持同步，避免增量更新后出现脏状态。
        self.hash = string_hash(&source);
        self.source = source;
    }

    /// 更新翻译字符串并重新计算哈希
    pub fn set_translation(&mut self, translation: String) {
        // hash_trans 用于快速判断译文是否发生变化。
        self.hash_trans = string_hash(&translation);
        self.translation = translation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sky_string_creation() {
        let sk = SkyString::new(
            1,
            "Hello".to_string(),
            "Bonjour".to_string(),
            *b"INFO",
            *b"DESC",
        );
        assert_eq!(sk.id, 1);
        assert_eq!(sk.source, "Hello");
        assert_eq!(sk.translation, "Bonjour");
        assert_eq!(sk.record_sig, *b"INFO");
        assert_eq!(sk.field_sig, *b"DESC");
        assert_ne!(sk.hash, 0);
        assert_ne!(sk.hash_trans, 0);
        assert_eq!(sk.colab_id, 0);
    }

    #[test]
    fn test_set_source_updates_hash() {
        let mut sk = SkyString::new(1, "Hello".to_string(), "".to_string(), *b"INFO", *b"DESC");
        let old_hash = sk.hash;
        sk.set_source("World".to_string());
        assert_ne!(sk.hash, old_hash);
        assert_eq!(sk.hash, string_hash("World"));
    }
}
