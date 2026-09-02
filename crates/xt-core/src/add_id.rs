//! FormID 批量偏移 / 重映射工具
//!
//! 对齐 Delphi 原版 `TESVT_AddId.pas`：
//! 允许对 ESP 记录中的 FormID / DialRef 进行批量偏移计算或掩码变换，
//! 支持指定范围（全部、仅未翻译、仅选中），主要用于 Mod 合并与 FormID 迁移场景。

use serde::{Deserialize, Serialize};

/// FormID 偏移范围
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddIdScope {
    /// 作用于全部条目
    All,
    /// 仅作用于未翻译条目
    UntranslatedOnly,
    /// 仅作用于选中条目
    SelectedOnly,
}

impl Default for AddIdScope {
    fn default() -> Self {
        Self::UntranslatedOnly
    }
}

/// FormID 偏移操作选项
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIdOptions {
    /// 基础偏移值（16 进制或十进制数值，如 0x01000000）
    pub offset_value: i64,
    /// 是否应用到 FormID 引用
    pub apply_to_form_id: bool,
    /// 作用范围
    pub scope: AddIdScope,
    /// 选中的 ID 列表（当 scope 为 SelectedOnly 时生效）
    pub selected_ids: Option<Vec<u32>>,
}

impl Default for AddIdOptions {
    fn default() -> Self {
        Self {
            offset_value: 0,
            apply_to_form_id: true,
            scope: AddIdScope::UntranslatedOnly,
            selected_ids: None,
        }
    }
}

/// FormID 计算结果摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIdResult {
    pub modified_count: u32,
    pub total_processed: u32,
}

/// 计算单个 FormID 偏移
pub fn calculate_offset_form_id(original_form_id: u32, offset: i64) -> u32 {
    let result = (original_form_id as i64) + offset;
    if result < 0 {
        0
    } else if result > u32::MAX as i64 {
        u32::MAX
    } else {
        result as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_offset() {
        assert_eq!(calculate_offset_form_id(0x00012345, 0x01000000), 0x01012345);
        assert_eq!(calculate_offset_form_id(0x02012345, -0x01000000), 0x01012345);
        assert_eq!(calculate_offset_form_id(0x00000010, -100), 0);
    }
}
