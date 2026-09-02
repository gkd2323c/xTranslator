//! Delphi `AddIdToStrings` 的 Rust 等价实现
//!
//! 对齐 `TESVT_main.pas::addIdToStringEx`：
//! - 按稳定 `u32 id` 更新 translation（不修改 ESP FormID）。
//! - 三档 scope：Everything / NoTransValid / Selection。
//! - 跳过 locked（PEX_NO_TRANS / locked VMAD）与 isempty（源文空且译文等于源文）。
//! - 四个可选前缀（checkbox a/b/c/d）按顺序拼接：
//!   a) `[%.5x]`  string ID（5 位 hex 小写，零填充）
//!   b) `[%.8x]`  FormID（8 位 hex 小写，零填充）
//!   c) `[REC:FIELD]` 记录类型与字段签名（4 字节 ASCII）
//!   d) `[@%.8x]` INFO 记录的 DIAL master FormID（需 record tree）
//! - 最终格式：`prefix + ' ' + 原译文`（即使 prefix 为空也保留前导空格，与 Delphi 一致）。

use crate::esp::record_tree::{EspFile, EspGrup};
use crate::types::params::SkyStringInternalParams;
use crate::types::sky_string::SkyString;
use std::collections::{HashMap, HashSet};

/// AddIdToStrings 作用范围
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AddIdToStringsScope {
    /// 0: 覆盖全部
    #[default]
    Everything,
    /// 1: 仅未翻译且未验证（Delphi `compareOptNoTransValid`）
    NoTransValid,
    /// 2: 仅选中项（Delphi `compareOptSelection`）
    Selection,
}

/// AddIdToStrings 选项
#[derive(Clone, Debug, Default)]
pub struct AddIdToStringsOptions {
    pub scope: AddIdToStringsScope,
    pub selected_ids: Option<HashSet<u32>>,
    /// a: 添加 String ID 前缀 `[%.5x]`
    pub add_string_id: bool,
    /// b: 添加 FormID 前缀 `[%.8x]`（Delphi 标签为 Add_EDID，实际格式化 formID）
    pub add_form_id: bool,
    /// c: 添加记录/字段引用 `[REC:FIELD]`（需 ESP 已加载）
    pub add_record_ref: bool,
    /// d: 添加 DIAL master 引用 `[@%.8x]`（仅 INFO 记录，需 record tree）
    pub add_dial_ref: bool,
}

/// 执行结果
#[derive(Clone, Debug, Default)]
pub struct AddIdToStringsResult {
    pub modified_count: u32,
    pub total_processed: u32,
}

/// 执行 AddIdToStrings：按选项为符合条件的字符串译文添加标识前缀。
///
/// # 参数
/// - `strings`: 目标字符串切片（按稳定 `u32 id` 定位）
/// - `esp_file`: 可选的已解析 ESP 文件（提供 record tree 用于 c/d 选项）
/// - `opts`: 前缀组合与范围选项
pub fn add_id_to_strings(
    strings: &mut [SkyString],
    esp_file: Option<&EspFile>,
    opts: &AddIdToStringsOptions,
) -> AddIdToStringsResult {
    let mut result = AddIdToStringsResult::default();

    // 预构建 INFO → DIAL master 映射（d 选项需要）
    let info_dial_map: HashMap<u32, u32> = esp_file.map(build_info_dial_map).unwrap_or_default();

    let has_any_prefix =
        opts.add_string_id || opts.add_form_id || opts.add_record_ref || opts.add_dial_ref;

    if !has_any_prefix {
        return result;
    }

    for sk in strings.iter_mut() {
        result.total_processed += 1;

        // scope 过滤
        if !is_in_scope(sk, opts) {
            continue;
        }

        // locked 跳过（PEX_NO_TRANS 或 locked VMAD）
        if is_locked(sk) {
            continue;
        }

        // isempty 跳过（源文空且译文等于源文）
        if is_empty(sk) {
            continue;
        }

        let mut prefix = String::new();

        // a: String ID `[%.5x]`（Delphi precision = minimum digits, zero-padded）
        if opts.add_string_id {
            prefix.push_str(&format!("[{:05x}]", sk.esp_ptr.str_id));
        }

        // b: FormID `[%.8x]`
        if opts.add_form_id {
            prefix.push_str(&format!("[{:08x}]", sk.esp_ptr.form_id));
        }

        // c: Record/Field `[REC:FIELD]`（需 ESP 关联）
        if opts.add_record_ref && is_esp_assigned(sk) {
            let rec = std::str::from_utf8(&sk.esp_ptr.record_sig).unwrap_or("????");
            let field = std::str::from_utf8(&sk.esp_ptr.field_sig).unwrap_or("????");
            prefix.push_str(&format!("[{}:{}]", rec, field));
        }

        // d: DIAL master ref `[@%.8x]`（仅 INFO 记录）
        if opts.add_dial_ref && is_esp_assigned(sk) && sk.esp_ptr.record_sig == *b"INFO" {
            if let Some(&dial_id) = info_dial_map.get(&sk.esp_ptr.form_id) {
                prefix.push_str(&format!("[@{:08x}]", dial_id));
            }
        }

        // Delphi: `sk1.strans := strtmp + ' ' + sk1.strans`
        // 即使 prefix 为空也保留前导空格（与 Delphi 行为一致）
        sk.translation = format!("{} {}", prefix, sk.translation);
        result.modified_count += 1;
    }

    result
}

fn is_in_scope(sk: &SkyString, opts: &AddIdToStringsOptions) -> bool {
    match opts.scope {
        AddIdToStringsScope::Everything => true,
        AddIdToStringsScope::NoTransValid => {
            !sk.params.is_translated() && !sk.params.is_validated()
        }
        AddIdToStringsScope::Selection => opts
            .selected_ids
            .as_ref()
            .map_or(false, |set| set.contains(&sk.id)),
    }
}

/// Delphi `lockedStatus`：PEX_NO_TRANS 或 locked VMAD。
fn is_locked(sk: &SkyString) -> bool {
    sk.internal_params
        .is_set(SkyStringInternalParams::PEX_NO_TRANS)
        || sk.params.is_locked()
}

/// Delphi `isEmpty`：`(gS = '') and (gS = gTrans)`。
fn is_empty(sk: &SkyString) -> bool {
    sk.source.is_empty() && sk.source == sk.translation
}

/// Delphi `isEspAssigned`：record/field 指针已分配。
/// Rust 代理：记录签名与字段签名均非空。
fn is_esp_assigned(sk: &SkyString) -> bool {
    sk.esp_ptr.record_sig != [0; 4] && sk.esp_ptr.field_sig != [0; 4]
}

/// 遍历 ESP record tree，构建 INFO form_id → DIAL master form_id 映射。
///
/// Delphi 逻辑：`INFO` 记录的父 GRUP 的 `sIdent` 即为 DIAL formID；
/// 再经 `getSubRec(dialID, headerDIAL)` 验证该 DIAL 记录存在。
fn build_info_dial_map(esp_file: &EspFile) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    for grup in &esp_file.top_level_grups {
        walk_grup(grup, &mut map);
    }
    map
}

fn walk_grup(grup: &EspGrup, map: &mut HashMap<u32, u32>) {
    // 若本 GRUP 包含 INFO 记录，且 s_ident 不是纯 ASCII 标签（即 children GRUP 的 label = formID）
    let has_info = grup.records.iter().any(|r| r.header.name == *b"INFO");
    if has_info && !is_ascii_label(&grup.grup_header.s_ident) {
        let dial_id = u32::from_le_bytes(grup.grup_header.s_ident);
        if dial_id != 0 {
            for rec in &grup.records {
                if rec.header.name == *b"INFO" {
                    map.insert(rec.form_id, dial_id);
                }
            }
        }
    }
    for child in &grup.children {
        walk_grup(child, map);
    }
}

fn is_ascii_label(bytes: &[u8; 4]) -> bool {
    bytes.iter().all(|&b| b.is_ascii_alphabetic() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::esp_pointer::EspPointer;
    use crate::types::params::SkyStringParams;

    fn make_sk(
        id: u32,
        source: &str,
        translation: &str,
        str_id: i32,
        form_id: u32,
        record_sig: [u8; 4],
        field_sig: [u8; 4],
    ) -> SkyString {
        let mut sk = SkyString::new(
            id,
            source.to_string(),
            translation.to_string(),
            record_sig,
            field_sig,
        );
        sk.esp_ptr = EspPointer {
            str_id,
            form_id,
            record_sig,
            field_sig,
            ..EspPointer::null()
        };
        sk
    }

    #[test]
    fn test_add_id_prefixes_all_combinations() {
        let sk = make_sk(0, "Hello", "你好", 42, 0x0001A4B2, *b"INFO", *b"NAM1");
        let mut strings = vec![sk];
        let opts = AddIdToStringsOptions {
            scope: AddIdToStringsScope::Everything,
            add_string_id: true,
            add_form_id: true,
            add_record_ref: true,
            add_dial_ref: false,
            ..Default::default()
        };
        let result = add_id_to_strings(&mut strings, None, &opts);
        assert_eq!(result.modified_count, 1);
        // [%.5x] = 0002a, [%.8x] = 0001a4b2, [INFO:NAM1]
        assert_eq!(strings[0].translation, "[0002a][0001a4b2][INFO:NAM1] 你好");
    }

    #[test]
    fn test_scope_notrans_valid_skips_translated() {
        let mut sk1 = make_sk(0, "A", "", 1, 0, *b"QUST", *b"FULL");
        sk1.params.set(SkyStringParams::TRANSLATED, true);
        let sk2 = make_sk(1, "B", "", 2, 0, *b"QUST", *b"FULL");
        let mut strings = vec![sk1, sk2];
        let opts = AddIdToStringsOptions {
            scope: AddIdToStringsScope::NoTransValid,
            add_string_id: true,
            ..Default::default()
        };
        let result = add_id_to_strings(&mut strings, None, &opts);
        assert_eq!(result.modified_count, 1);
        assert_eq!(strings[0].translation, ""); // skipped
        assert_eq!(strings[1].translation, "[00002] ");
    }

    #[test]
    fn test_locked_and_empty_skipped() {
        let mut sk1 = make_sk(0, "A", "a", 1, 0, *b"QUST", *b"FULL");
        sk1.internal_params
            .set(SkyStringInternalParams::PEX_NO_TRANS, true);
        let sk2 = make_sk(1, "", "", 2, 0, *b"QUST", *b"FULL");
        let mut strings = vec![sk1, sk2];
        let opts = AddIdToStringsOptions {
            scope: AddIdToStringsScope::Everything,
            add_string_id: true,
            ..Default::default()
        };
        let result = add_id_to_strings(&mut strings, None, &opts);
        assert_eq!(result.modified_count, 0);
        assert_eq!(strings[0].translation, "a");
        assert_eq!(strings[1].translation, "");
    }

    #[test]
    fn test_selection_scope() {
        let sk1 = make_sk(0, "A", "a", 1, 0, *b"QUST", *b"FULL");
        let sk2 = make_sk(1, "B", "b", 2, 0, *b"QUST", *b"FULL");
        let mut strings = vec![sk1, sk2];
        let mut selected = HashSet::new();
        selected.insert(1);
        let opts = AddIdToStringsOptions {
            scope: AddIdToStringsScope::Selection,
            selected_ids: Some(selected),
            add_string_id: true,
            ..Default::default()
        };
        add_id_to_strings(&mut strings, None, &opts);
        assert_eq!(strings[0].translation, "a");
        assert_eq!(strings[1].translation, "[00002] b");
    }

    #[test]
    fn test_no_prefix_does_nothing() {
        let sk = make_sk(0, "A", "a", 1, 0, *b"QUST", *b"FULL");
        let mut strings = vec![sk];
        let opts = AddIdToStringsOptions {
            scope: AddIdToStringsScope::Everything,
            ..Default::default()
        };
        let result = add_id_to_strings(&mut strings, None, &opts);
        assert_eq!(result.modified_count, 0);
        assert_eq!(strings[0].translation, "a");
    }
}
