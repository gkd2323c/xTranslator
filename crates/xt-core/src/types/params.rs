/// SkyString 状态参数（持久化标志位）
///
/// 对应 Delphi 的 `sStrParams` 集合类型，存储为 1 字节（u8）
/// 这些标志位会持久化到 SST 字典文件中
/// 注意：这些位用于跨会话状态恢复，调整语义需要兼容旧 SST。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkyStringParams(pub u8);

impl SkyStringParams {
    /// 0x01 - 已完全翻译（UI显示为白色）
    pub const TRANSLATED: u8 = 1 << 0;
    /// 0x02 - 锁定翻译（UI显示为黄色，不允许修改）
    pub const LOCKED_TRANS: u8 = 1 << 1;
    /// 0x04 - 部分翻译（UI显示为粉色，译文不完整）
    pub const INCOMPLETE_TRANS: u8 = 1 << 2;
    /// 0x08 - 已验证（UI显示为蓝色，译文已校对）
    pub const VALIDATED: u8 = 1 << 3;
    /// 0x10 - 已废弃参数1（保留，未使用）
    pub const DEPRECATED_PARAM1: u8 = 1 << 4;
    /// 0x20 - 已废弃参数2（保留，未使用）
    pub const DEPRECATED_PARAM2: u8 = 1 << 5;
    /// 0x40 - 从 SST 加载但尚未应用到 ESP 的旧数据
    pub const OLD_DATA: u8 = 1 << 6;
    /// 0x80 - 未翻译但分配了协作 ID
    pub const PENDING: u8 = 1 << 7;

    /// 创建新的空参数集合（所有标志位清零）
    pub fn new() -> Self {
        Self(0)
    }

    /// 检查指定的标志位是否已设置
    pub fn is_set(&self, flag: u8) -> bool {
        // 非零即命中，支持组合位检查。
        self.0 & flag != 0
    }

    /// 设置或清除指定的标志位
    pub fn set(&mut self, flag: u8, value: bool) {
        if value {
            self.0 |= flag; // 设置位
        } else {
            self.0 &= !flag; // 清除位
        }
    }

    /// 是否已完全翻译
    pub fn is_translated(&self) -> bool {
        self.is_set(Self::TRANSLATED)
    }
    /// 是否锁定（不允许修改）
    pub fn is_locked(&self) -> bool {
        self.is_set(Self::LOCKED_TRANS)
    }
    /// 是否部分翻译（译文不完整）
    pub fn is_incomplete(&self) -> bool {
        self.is_set(Self::INCOMPLETE_TRANS)
    }
    /// 是否已验证（译文已校对）
    pub fn is_validated(&self) -> bool {
        self.is_set(Self::VALIDATED)
    }
    /// 是否为从 SST 加载的旧数据（未应用到当前 ESP）
    pub fn is_old_data(&self) -> bool {
        self.is_set(Self::OLD_DATA)
    }
    /// 是否为待处理状态（未翻译但有协作 ID）
    pub fn is_pending(&self) -> bool {
        self.is_set(Self::PENDING)
    }
}

/// SkyString 内部参数（运行时标志位，不持久化）
///
/// 对应 Delphi 的 `sStrInternalParams` 集合类型
/// 这些标志位仅在内存中使用，不会保存到 SST 文件中
/// 使用 u64 可容纳 Delphi 原版使用的约 45 个标志位
/// 约定：序列化/导出逻辑不应依赖这些位作为唯一数据来源。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkyStringInternalParams(pub u64);

impl SkyStringInternalParams {
    // ===== 匹配和查找相关 =====
    /// 0x01 - 已找到匹配的 ID（用于查找重复项）
    pub const MATCHED_ID_FOUND: u64 = 1 << 0;
    /// 0x02 - 标记为已删除
    pub const IS_DELETED: u64 = 1 << 1;
    /// 0x04 - 单词字符串（用于启发式搜索优化）
    pub const IS_ONE_WORD: u64 = 1 << 2;
    /// 0x08 - 低优先级警告
    pub const LOW_WARNING: u64 = 1 << 3;
    /// 0x10 - 一般警告
    pub const WARNING: u64 = 1 << 4;
    /// 0x20 - 严重警告
    pub const BIG_WARNING: u64 = 1 << 5;
    /// 0x40 - 多个翻译变体
    pub const N_TRANS: u64 = 1 << 6;
    /// 0x80 - 缓存条目（从缓存加载而非原始数据）
    pub const IS_CACHE: u64 = 1 << 7;

    // ===== 导出和本地化相关 =====
    /// 0x100 - 需要导出到本地化 ESP
    pub const TO_LOCALIZED_ESP: u64 = 1 << 8;
    /// 0x200 - 需要导出到共享 ID
    pub const TO_LOCALIZED_SHARED_ID: u64 = 1 << 9;

    // ===== 初始化状态 =====
    /// 0x400 - 源字符串已初始化
    pub const IS_SOURCE_INITIALIZED: u64 = 1 << 10;
    /// 0x800 - 翻译字符串已初始化
    pub const IS_TRANS_INITIALIZED: u64 = 1 << 11;

    // ===== 错误和验证 =====
    /// 0x1000 - 别名错误
    pub const ALIAS_ERROR: u64 = 1 << 12;
    /// 0x2000 - 严格别名检查
    pub const HAS_ALIAS_STRICT: u64 = 1 << 13;
    /// 0x4000 - 包含数字
    pub const HAS_NUMBER: u64 = 1 << 14;
    /// 0x8000 - 孤立字符串（无引用）
    pub const IS_ORPHEAN: u64 = 1 << 15;
    /// 0x10000 - 查找失败
    pub const IS_LOOKUP_FAILED: u64 = 1 << 16;
    /// 0x20000 - 未经授权的行分隔符
    pub const UNAUTH_LINE_BREAK: u64 = 1 << 17;
    /// 0x40000 - 字符串大小错误
    pub const STRING_SIZE_ERROR: u64 = 1 << 18;
    /// 0x80000 - 回车符错误
    pub const STRING_CR_ERROR: u64 = 1 << 19;

    // ===== 显示和样式 =====
    /// 0x100000 - 替代颜色显示
    pub const ALT_COLOR: u64 = 1 << 20;
    /// 0x200000 - PEX 脚本无翻译
    pub const PEX_NO_TRANS: u64 = 1 << 21;
    /// 0x400000 - PEX 脚本警告
    pub const PEX_WARN: u64 = 1 << 22;
    /// 0x800000 - 拼写检查错误
    pub const SPELL_CHECK_FAULT: u64 = 1 << 23;

    // ===== SST 相关 =====
    /// 0x1000000 - 未在 SST 中使用
    pub const UNUSED_IN_SST: u64 = 1 << 24;
    /// 0x2000000 - 模糊匹配警告
    pub const FUZ_WARNING: u64 = 1 << 25;
    /// 0x4000000 - 女性角色对话
    pub const IS_FEMALE: u64 = 1 << 26;
    /// 0x8000000 - VMAD 脚本字符串
    pub const IS_VMAD_STRING: u64 = 1 << 27;
    /// 0x10000000 - SST 已应用
    pub const SST_APPLIED: u64 = 1 << 28;
    /// 0x20000000 - NPC 警告
    pub const NPC_WARNING: u64 = 1 << 29;
    /// 0x40000000 - 源字符串白色标记
    pub const SOURCE_WHITE: u64 = 1 << 30;
    /// 0x80000000 - 翻译字符串白色标记
    pub const TRANS_WHITE: u64 = 1 << 31;

    // ===== 高级功能 =====
    /// 0x100000000 - 派生计算值
    pub const DERIVED_COMPUTED: u64 = 1 << 32;
    /// 0x200000000 - SST 中所有未使用
    pub const ALL_UNUSED_IN_SST: u64 = 1 << 33;
    /// 0x400000000 - 在翻译 API 数组中
    pub const ON_TRANSLATION_API_ARRAY: u64 = 1 << 34;
    /// 0x800000000 - 在翻译 API 数组预选
    pub const ON_TRANSLATION_API_ARRAY_PRESELECTION: u64 = 1 << 35;
    /// 0x1000000000 - 翻译软锁定
    pub const ON_TRANSLATION_SOFT_LOCK: u64 = 1 << 36;
    /// 0x2000000000 - 翻译 CRLF 数组
    pub const ON_TRANSLATION_CRLF_ARRAY: u64 = 1 << 37;
    /// 0x4000000000 - 翻译重试
    pub const ON_TRANSLATION_RETRY: u64 = 1 << 38;
    /// 0x8000000000 - 翻译 API 数组块1
    pub const ON_TRANSLATION_API_ARRAY_BLOCK1: u64 = 1 << 39;
    /// 0x10000000000 - 翻译 API 数组块2
    pub const ON_TRANSLATION_API_ARRAY_BLOCK2: u64 = 1 << 40;
    /// 0x20000000000 - 字符串 ID 已变更
    pub const STRING_ID_CHANGED: u64 = 1 << 41;
    /// 0x40000000000 - 不标记无翻译
    pub const DO_NOT_TAG_NO_TRANS: u64 = 1 << 42;
    /// 0x80000000000 - 版本4格式
    pub const IS_VERSION4: u64 = 1 << 43;
    /// 0x100000000000 - 已规范化
    pub const IS_NORMALIZED: u64 = 1 << 44;

    /// 创建新的空内部参数集合
    pub fn new() -> Self {
        Self(0)
    }

    /// 检查指定的标志位是否已设置
    pub fn is_set(&self, flag: u64) -> bool {
        self.0 & flag != 0
    }

    /// 设置或清除指定的标志位
    pub fn set(&mut self, flag: u64, value: bool) {
        if value {
            self.0 |= flag; // 设置位
        } else {
            self.0 &= !flag; // 清除位
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_flags() {
        let mut p = SkyStringParams::new();
        assert!(!p.is_translated());
        p.set(SkyStringParams::TRANSLATED, true);
        assert!(p.is_translated());
        p.set(SkyStringParams::TRANSLATED, false);
        assert!(!p.is_translated());
    }

    #[test]
    fn test_internal_params_flags() {
        let mut p = SkyStringInternalParams::new();
        assert!(!p.is_set(SkyStringInternalParams::IS_NORMALIZED));
        p.set(SkyStringInternalParams::IS_NORMALIZED, true);
        assert!(p.is_set(SkyStringInternalParams::IS_NORMALIZED));
    }
}
