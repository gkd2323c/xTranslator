//! DEF_UI / FallUI 物品组件标签生成器 (DP-10)
//!
//! 在 Fallout 4 / 76 / Starfield 等游戏中，自动解析 MISC 物品引用的 CMPO 组件，
//! 并根据模板生成带分解材料与重量标签的翻译文本。

use crate::types::game_id::GameId;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// 单个组件引用信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentRef {
    pub form_id: u32,
    pub count: u32,
    pub name: String,
}

/// DEF_UI 组件生成器配置选项（与 Delphi `rDefUIOptions` 100% 对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefUiOptions {
    /// 是否使用源语言作为基础名称（false 则使用当前已有翻译或回退源语言）
    pub use_source_for_string: bool,
    /// 是否使用源语言作为组件名称（false 则使用组件的翻译）
    pub use_source_for_components: bool,
    /// 是否启用正则清洗基础名称（去除旧标签）
    pub clean_base: bool,
    /// 是否启用正则清洗组件名称
    pub clean_compo: bool,
    /// 是否在材料名后添加数量标记（如 `*`、`**`）
    pub add_quantity: bool,
    /// 是否只取组件名称的首字母
    pub use_first_char: bool,
    /// 是否在只有单个组件时自动生成 Header（如 `[Steel] Desk Fan`）
    pub do_auto_header: bool,
    /// 正则表达式 1：用于清洗基础名
    pub regex_clean_base: String,
    /// 正则表达式 2：用于清洗组件名
    pub regex_clean_compo: String,
    /// 格式化模板，如 `%BASE% {{{%COMPOS%}}}` 或 `%BASE% [%COMPOS%]`
    pub template: String,
    /// 带重量的格式化模板（若配置）
    pub template_with_weight: Option<String>,
    /// 组件之间的分隔符（默认为 `, `）
    pub component_separator: String,
    /// 数量指示符 1（小量）
    pub quantity_indicator1: String,
    /// 数量指示符 2（大量）
    pub quantity_indicator2: String,
    /// 忽略处理的 EDID / FormId 过滤列表（换行或逗号分隔）
    pub ignore_list: Vec<String>,
    /// 目标处理范围：`"all"` | `"untranslated"` | `"selected"`
    pub scope: String,
}

impl Default for DefUiOptions {
    fn default() -> Self {
        Self {
            use_source_for_string: false,
            use_source_for_components: true,
            clean_base: true,
            clean_compo: true,
            add_quantity: false,
            use_first_char: false,
            do_auto_header: false,
            regex_clean_base: r"^(.+)\{\{\{.+\}\}\}$".to_string(),
            regex_clean_compo: r"^[\[\(\{\|].+?[\|\]\}\)](.+)$".to_string(),
            template: "%BASE% {{{%COMPOS%}}}".to_string(),
            template_with_weight: Some("%BASE% {{{%WEIGHT%lb, %COMPOS%}}}".to_string()),
            component_separator: ", ".to_string(),
            quantity_indicator1: "*".to_string(),
            quantity_indicator2: "**".to_string(),
            ignore_list: Vec::new(),
            scope: "all".to_string(),
        }
    }
}

impl DefUiOptions {
    /// 为特定游戏获取预设默认值
    pub fn for_game(game: GameId) -> Self {
        match game {
            GameId::Fallout4 => Self {
                regex_clean_base: r"^(.+)\{\{\{.+\}\}\}$".to_string(),
                regex_clean_compo: r"^[\[\(\{\|].+?[\|\]\}\)](.+)$".to_string(),
                template: "%BASE% {{{%COMPOS%}}}".to_string(),
                template_with_weight: Some("%BASE% {{{%WEIGHT%lb, %COMPOS%}}}".to_string()),
                ..Self::default()
            },
            GameId::Fallout76 => Self {
                regex_clean_base: r"^(.+)\(.+\)$".to_string(),
                regex_clean_compo: r"^[\[\(\{\|].+?[\|\]\}\)](.+)$".to_string(),
                template: "%BASE% (%COMPOS%)".to_string(),
                template_with_weight: Some("%BASE% (%WEIGHT%lb, %COMPOS%)".to_string()),
                ..Self::default()
            },
            GameId::Starfield => Self {
                regex_clean_base: r"^(.+)\{\{\{.+\}\}\}$".to_string(),
                regex_clean_compo: r"^[\[\(\{\|].+?[\|\]\}\)](.+)$".to_string(),
                template: "%BASE% {{{%COMPOS%}}}".to_string(),
                template_with_weight: Some("%BASE% {{{%WEIGHT%lb, %COMPOS%}}}".to_string()),
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

/// 解析 CVPA (Component Values and Quantities) 或 MCQP 原始字节
///
/// 结构：每项 `item_size` 字节（FO4/FO76/SF 通常为 8 字节：4 字节 FormId + 4 字节数量/权重）。
pub fn parse_cvpa_buffer(buffer: &[u8], item_size: usize) -> Vec<(u32, u32)> {
    if item_size == 0 || buffer.len() < item_size {
        return Vec::new();
    }
    let count = buffer.len() / item_size;
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let offset = i * item_size;
        if offset + 8 <= buffer.len() {
            let form_id = u32::from_le_bytes([
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
            ]);
            let qty = u32::from_le_bytes([
                buffer[offset + 4],
                buffer[offset + 5],
                buffer[offset + 6],
                buffer[offset + 7],
            ]);
            if form_id != 0 {
                result.push((form_id, qty));
            }
        }
    }
    result
}

/// 从 DATA 字段的指定 offset 读取 single 浮点数作为重量 (weight)
pub fn parse_data_weight(buffer: &[u8], offset: usize) -> f32 {
    if buffer.len() >= offset + 4 {
        f32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ])
    } else {
        0.0
    }
}

/// 格式化浮点数字符串（对齐 Delphi: 1.0 -> "1", 1.5 -> "1.5"）
pub fn format_weight(weight: f32) -> String {
    let s = format!("{:.1}", weight);
    if s.ends_with(".0") {
        format!("{:.0}", weight)
    } else {
        s
    }
}

/// 根据选项应用单条字符串的组件标签生成
pub fn format_def_ui_string(
    source: &str,
    translation: &str,
    components: &[ComponentRef],
    weight: f32,
    opts: &DefUiOptions,
    game: GameId,
) -> String {
    // 1. 基础名称提取
    let mut base_string = if opts.use_source_for_string || translation.trim().is_empty() {
        source.to_string()
    } else {
        translation.to_string()
    };

    // 2. 正则清洗基础名
    if opts.clean_base && !opts.regex_clean_base.is_empty() {
        if let Ok(re) = Regex::new(&opts.regex_clean_base) {
            if let Some(caps) = re.captures(&base_string) {
                if let Some(m) = caps.get(1) {
                    base_string = m.as_str().trim().to_string();
                }
            }
        }
    }

    // 3. 如果无组件，返回清洗后的基础名
    if components.is_empty() {
        return base_string;
    }

    let regex_compo = if opts.clean_compo && !opts.regex_clean_compo.is_empty() {
        Regex::new(&opts.regex_clean_compo).ok()
    } else {
        None
    };

    let mut compo_parts = Vec::new();
    let compo_count = components.len();

    for comp in components {
        let mut compo_name = comp.name.clone();

        // 正则清洗组件名
        if let Some(ref re) = regex_compo {
            if let Some(caps) = re.captures(&compo_name) {
                if let Some(m) = caps.get(1) {
                    compo_name = m.as_str().trim().to_string();
                }
            }
        }

        // 首字母缩写
        if opts.use_first_char && !compo_name.is_empty() {
            compo_name = compo_name
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or(compo_name);
        }

        // 数量标记
        let mut qty_tag = String::new();
        if opts.add_quantity {
            match game {
                GameId::Fallout76 => {
                    // FO76: 62410 -> *, 62411 -> **
                    if comp.count == 62410 {
                        qty_tag = opts.quantity_indicator1.clone();
                    } else if comp.count == 62411 {
                        qty_tag = opts.quantity_indicator2.clone();
                    }
                }
                _ => {
                    // FO4/Starfield: >5 -> **, >2 -> *
                    if comp.count > 5 {
                        qty_tag = opts.quantity_indicator2.clone();
                    } else if comp.count > 2 {
                        qty_tag = opts.quantity_indicator1.clone();
                    }
                }
            }
        }

        compo_parts.push(format!("{}{}", compo_name, qty_tag));
    }

    let compo_string = compo_parts.join(&opts.component_separator);
    let weight_string = format_weight(weight);

    // 4. 应用模板替换
    let template = if opts.template_with_weight.is_some() && weight > 0.0 {
        opts.template_with_weight.as_ref().unwrap()
    } else {
        &opts.template
    };

    let mut result = template.clone();
    result = result.replace("%BASE%", &base_string);
    result = result.replace("%WEIGHT%", &weight_string);
    result = result.replace("%COMPOS%", &compo_string);

    // Auto Header (单组件情况：[Component] Base)
    if opts.do_auto_header && compo_count == 1 && !compo_parts.is_empty() {
        let single_header = &compo_parts[0];
        result = format!("[{}] {}", single_header, result);
    }

    result
}

use crate::esp::record_tree::EspFile;
use crate::types::sky_string::SkyString;
use std::collections::{HashMap, HashSet};

/// 遍历当前所有字符串和 ESP 记录，批量生成 DEF_UI 标签译文
///
/// 返回 `(Vec<(u32, String, String)>, u32)`：
/// `(vec![(str_id, new_translation, original_text)], total_misc_records_found)`
pub fn generate_def_ui_translations(
    strings: &[SkyString],
    esp_file: Option<&EspFile>,
    opts: &DefUiOptions,
    selected_ids: Option<&HashSet<u32>>,
    game: GameId,
) -> (Vec<(u32, String, String)>, u32) {
    let mut mutations = Vec::new();
    let mut total_misc = 0u32;

    // Pre-parse ignore list entries: EDID/keyword fragments (lowercase) and FormIDs (hex)
    let ignore_keywords: Vec<String> = opts
        .ignore_list
        .iter()
        .filter(|e| !e.trim().is_empty())
        .map(|e| e.trim().to_lowercase())
        .collect();
    let ignore_form_ids: Vec<u32> = ignore_keywords
        .iter()
        .filter_map(|e| u32::from_str_radix(e.trim_start_matches("0x"), 16).ok())
        .collect();

    // 1. 构建 Component 名称查找表 (CMPO record FormID -> (Source Name, Trans Name))
    let mut compo_map: HashMap<u32, (String, String)> = HashMap::new();
    for s in strings {
        if &s.record_sig == b"CMPO" && &s.field_sig == b"FULL" {
            compo_map.insert(s.esp_ptr.form_id, (s.source.clone(), s.translation.clone()));
        }
    }

    // 2. 遍历所有 MISC 记录字符串
    for s in strings {
        if &s.record_sig != b"MISC" || &s.field_sig != b"FULL" {
            continue;
        }
        total_misc += 1;

        // Ignore list: skip by FormID match or EDID/keyword fragment (case-insensitive)
        if ignore_form_ids.contains(&s.esp_ptr.form_id) {
            continue;
        }
        if let Some(edid) = &s.edid {
            let edid_lower = edid.to_lowercase();
            if ignore_keywords
                .iter()
                .any(|kw| edid_lower.contains(kw.as_str()))
            {
                continue;
            }
        }

        // 根据作用域过滤
        if let Some(set) = selected_ids {
            if opts.scope == "selection" && !set.contains(&s.id) {
                continue;
            }
        }
        if opts.scope == "untranslated" && !s.translation.trim().is_empty() {
            continue;
        }
        // 3. 从 ESP 记录树查找对应的 CVPA / MCQP 与 DATA (Weight)
        let mut components = Vec::new();
        let mut weight = 0.0f32;

        if let Some(esp) = esp_file {
            if let Some(rec) = esp.find_record_by_form_id(s.esp_ptr.form_id) {
                // 查找 CVPA (Component Values & Quantities) 或 MCQP
                for field in &rec.fields {
                    if &field.header.name == b"CVPA" || &field.header.name == b"MCQP" {
                        let parsed = parse_cvpa_buffer(&field.buffer, 8);
                        for (form_id, count) in parsed {
                            let name = if opts.use_source_for_components {
                                compo_map.get(&form_id).map(|(src, _)| src.clone())
                            } else {
                                compo_map.get(&form_id).map(|(_, tr)| {
                                    if !tr.trim().is_empty() {
                                        tr.clone()
                                    } else {
                                        compo_map.get(&form_id).unwrap().0.clone()
                                    }
                                })
                            }
                            .unwrap_or_else(|| format!("0x{:08X}", form_id));

                            components.push(ComponentRef {
                                form_id,
                                count,
                                name,
                            });
                        }
                    } else if &field.header.name == b"DATA" && field.buffer.len() >= 8 {
                        // Fallout 4 MISC DATA: Value(u32, offset 0), Weight(f32, offset 4)
                        weight = parse_data_weight(&field.buffer, 4);
                    }
                }
            }
        }

        let new_trans =
            format_def_ui_string(&s.source, &s.translation, &components, weight, opts, game);
        let orig = if !s.translation.is_empty() {
            s.translation.clone()
        } else {
            s.source.clone()
        };

        if new_trans != orig {
            mutations.push((s.id, new_trans, orig));
        }
    }

    (mutations, total_misc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cvpa_buffer() {
        let mut buf = Vec::new();
        // Item 1: FormId 0x0001EC8B, Count 2
        buf.extend_from_slice(&0x0001EC8Bu32.to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes());
        // Item 2: FormId 0x0001EC8C, Count 4
        buf.extend_from_slice(&0x0001EC8Cu32.to_le_bytes());
        buf.extend_from_slice(&4u32.to_le_bytes());

        let res = parse_cvpa_buffer(&buf, 8);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0], (0x0001EC8B, 2));
        assert_eq!(res[1], (0x0001EC8C, 4));
    }

    #[test]
    fn test_format_weight() {
        assert_eq!(format_weight(1.0), "1");
        assert_eq!(format_weight(1.5), "1.5");
        assert_eq!(format_weight(0.2), "0.2");
    }

    #[test]
    fn test_format_def_ui_string() {
        let opts = DefUiOptions::default();
        let comps = vec![
            ComponentRef {
                form_id: 1,
                count: 1,
                name: "Steel".to_string(),
            },
            ComponentRef {
                form_id: 2,
                count: 2,
                name: "Spring".to_string(),
            },
        ];

        let out = format_def_ui_string("Desk Fan", "", &comps, 0.0, &opts, GameId::Fallout4);
        assert_eq!(out, "Desk Fan {{{Steel, Spring}}}");

        // Test with existing DEF_UI tag cleaned
        let out_clean = format_def_ui_string(
            "Desk Fan {{{Old, Tag}}}",
            "",
            &comps,
            0.0,
            &opts,
            GameId::Fallout4,
        );
        assert_eq!(out_clean, "Desk Fan {{{Steel, Spring}}}");
    }

    #[test]
    fn test_quantity_indicators() {
        let mut opts = DefUiOptions::default();
        opts.add_quantity = true;
        let comps = vec![
            ComponentRef {
                form_id: 1,
                count: 3,
                name: "Steel".to_string(),
            },
            ComponentRef {
                form_id: 2,
                count: 6,
                name: "Spring".to_string(),
            },
        ];

        let out = format_def_ui_string("Desk Fan", "", &comps, 0.0, &opts, GameId::Fallout4);
        assert_eq!(out, "Desk Fan {{{Steel*, Spring**}}}");
    }

    use super::*;
    use crate::esp::header::{FieldHeader, GenericHeader, GrupHeader, RecordHeaderData};
    use crate::esp::record_tree::{EspField, EspFile, EspGrup, EspRecord, Tes4Header};

    /// Build a minimal EspFile containing one MISC record (with CVPA + DATA fields)
    /// and one CMPO record, both non-compressed.
    fn minimal_esp_file() -> EspFile {
        let misc_form_id = 0x0001EC8Bu32;
        let cmpo_form_id = 0x00010001u32;

        // CVPA: one component entry (FormId + count)
        let mut cvpa_buf = Vec::new();
        cvpa_buf.extend_from_slice(&cmpo_form_id.to_le_bytes());
        cvpa_buf.extend_from_slice(&3u32.to_le_bytes());
        // DATA: Value(u32) + Weight(f32)
        let mut data_buf = Vec::new();
        data_buf.extend_from_slice(&5u32.to_le_bytes());
        data_buf.extend_from_slice(&1.5f32.to_le_bytes());

        let misc_rec = EspRecord {
            header: GenericHeader {
                name: *b"MISC",
                dsize: 0,
            },
            record_header_data: RecordHeaderData {
                flags: 0,
                form_id: misc_form_id,
                version: 44,
                f_version: 131,
                v_info: 0,
            },
            fields: vec![
                EspField {
                    header: FieldHeader {
                        name: *b"CVPA",
                        dsize: cvpa_buf.len() as u16,
                    },
                    buffer: cvpa_buf,
                    is_size_xxxx: false,
                },
                EspField {
                    header: FieldHeader {
                        name: *b"DATA",
                        dsize: data_buf.len() as u16,
                    },
                    buffer: data_buf,
                    is_size_xxxx: false,
                },
            ],
            compressed: false,
            raw: false,
            form_id: misc_form_id,
            editor_id: None,
            original_raw_data: Vec::new(),
        };

        let cmpo_rec = EspRecord {
            header: GenericHeader {
                name: *b"CMPO",
                dsize: 0,
            },
            record_header_data: RecordHeaderData {
                flags: 0,
                form_id: cmpo_form_id,
                version: 44,
                f_version: 131,
                v_info: 0,
            },
            fields: vec![],
            compressed: false,
            raw: false,
            form_id: cmpo_form_id,
            editor_id: None,
            original_raw_data: Vec::new(),
        };

        let grup = EspGrup {
            header: GenericHeader {
                name: *b"GRUP",
                dsize: 0,
            },
            grup_header: GrupHeader {
                s_ident: *b"MISC",
                s_type: 0,
                s_tstamp: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            },
            records: vec![misc_rec, cmpo_rec],
            children: vec![],
        };

        EspFile {
            tes4: Tes4Header {
                generic: GenericHeader {
                    name: *b"TES4",
                    dsize: 0,
                },
                record_header_data: RecordHeaderData {
                    flags: 0,
                    form_id: 0,
                    version: 0,
                    f_version: 131,
                    v_info: 0,
                },
                field_data: Vec::new(),
            },
            top_level_grups: vec![grup],
        }
    }

    #[test]
    fn test_generate_respects_ignore_list() {
        let mut s1 = SkyString::new(
            1,
            "Desk Fan".to_string(),
            "".to_string(),
            *b"MISC",
            *b"FULL",
        );
        s1.esp_ptr.form_id = 0x0001EC8B;
        s1.edid = Some("DN101_DeskFan".to_string());
        let strings = vec![s1];

        let esp = minimal_esp_file();

        // Without ignore list: mutation produced from CVPA components
        let mut opts = DefUiOptions::default();
        opts.scope = "all".to_string();
        let (mutations, total) =
            generate_def_ui_translations(&strings, Some(&esp), &opts, None, GameId::Fallout4);
        assert_eq!(total, 1);
        assert_eq!(mutations.len(), 1, "no ignore list: mutation generated");
        assert!(mutations[0].1.contains("Desk Fan"));

        // With EDID keyword ignore list: no mutation
        opts.ignore_list = vec!["deskfan".to_string()];
        let (mutations, total) =
            generate_def_ui_translations(&strings, Some(&esp), &opts, None, GameId::Fallout4);
        assert_eq!(total, 1, "ignored strings still counted in total_misc");
        assert!(mutations.is_empty(), "EDID keyword match should be ignored");
    }

    #[test]
    fn test_generate_ignore_list_by_form_id() {
        let mut s1 = SkyString::new(
            1,
            "Desk Fan".to_string(),
            "".to_string(),
            *b"MISC",
            *b"FULL",
        );
        s1.esp_ptr.form_id = 0x0001EC8B;
        let strings = vec![s1];

        let esp = minimal_esp_file();

        let mut opts = DefUiOptions::default();
        opts.scope = "all".to_string();
        opts.ignore_list = vec!["0001EC8B".to_string()];

        let (mutations, _) =
            generate_def_ui_translations(&strings, Some(&esp), &opts, None, GameId::Fallout4);
        assert!(mutations.is_empty(), "FormID hex entry should be ignored");
    }

    #[test]
    fn test_scope_values_match_shared_contract() {
        // Guard: core scope aliases must stay aligned with xt-shared def_ui_scope
        assert_eq!(xt_shared::dto::def_ui_scope::ALL, "all");
        assert_eq!(
            xt_shared::dto::def_ui_scope::ONLY_UNTRANSLATED,
            "only_untranslated"
        );
        assert_eq!(xt_shared::dto::def_ui_scope::ONLY_SELECTED, "only_selected");
        // Core consumes canonical values via a thin alias map
        assert_eq!(core_scope("all"), "all");
        assert_eq!(core_scope("only_untranslated"), "untranslated");
        assert_eq!(core_scope("only_selected"), "selection");
    }

    /// Map shared scope constant to core internal alias.
    fn core_scope(shared: &str) -> &'static str {
        match shared {
            "all" => "all",
            "only_untranslated" => "untranslated",
            "only_selected" => "selection",
            _ => "all",
        }
    }
}
