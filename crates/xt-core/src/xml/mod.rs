use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::types::esp_pointer::{EspPointer, HeaderSig};
use crate::types::params::SkyStringInternalParams;
use crate::types::params::SkyStringParams;
use crate::types::sky_string::SkyString;

/// Delphi xTranslator 的 XML 导出格式解析器
///
/// XML 是 xTranslator 的通用交换格式，用于：
/// - 导出翻译供其他工具使用
/// - 导入来自其他工具的翻译
/// - 与翻译人员共享翻译工作
///
/// 格式示例：
/// ```xml
/// <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
/// <SSTXMLRessources>
///   <Params>
///     <Addon>Skyrim</Addon>
///     <Source>english</Source>
///     <Dest>chinese</Dest>
///     <Version>2</Version>
///   </Params>
///   <Content>
///     <String List="0" sID="000001">
///       <EDID>DialogueGenericHello</EDID>
///       <REC id="0" idMax="0">INFO:RNAM</REC>
///       <Source>Hello there</Source>
///       <Dest>你好</Dest>
///     </String>
///   </Content>
/// </SSTXMLRessources>
/// ```

#[derive(Debug, Clone)]
pub struct XmlExportParams {
    /// 插件名（如 "Skyrim", "Dawnguard"）
    pub addon: String,
    /// 源语言（如 "english"）
    pub source_lang: String,
    /// 目标语言（如 "chinese"）
    pub dest_lang: String,
    /// 格式版本（通常为 2）
    pub version: u32,
}

#[derive(Debug, Clone)]
pub struct XmlStringEntry {
    /// Strings 文件类型索引：0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
    pub list_index: u8,
    /// Strings 文件中的字符串 ID（T1 匹配的关键）
    pub str_id: i32,
    /// Editor ID（编辑器 ID，用于 T2 匹配）
    pub edid: Option<String>,
    /// 记录类型签名（如 "INFO", "DIAL"）
    pub record_sig: HeaderSig,
    /// 字段签名（如 "RNAM", "FULL"）
    pub field_sig: HeaderSig,
    /// 字符串在 Strings 文件中的索引（来自 REC 的 id 属性）
    pub index: u16,
    /// 最大索引值（来自 REC 的 idMax 属性，用于验证）
    pub index_max: u16,
    /// 源文本（原文）
    pub source: String,
    /// 翻译文本（译文）
    pub translation: String,
}

impl XmlStringEntry {
    /// 转换为 SkyString（仅填充最小必要的 EspPointer 信息）
    ///
    /// 注意：XML 导入时，form_id 和 edid_hash 无法从 XML 中获取，
    /// 这些信息需要在 T1-T4 匹配时从 ESP 中补充。
    pub fn to_sky_string(&self, id: u32) -> SkyString {
        let mut sk = SkyString::new(
            id,
            self.source.clone(),
            self.translation.clone(),
            self.record_sig,
            self.field_sig,
        );
        sk.esp_ptr = EspPointer {
            str_id: self.str_id,
            form_id: 0, // XML 中无法获取，需要后续补充
            record_sig: self.record_sig,
            field_sig: self.field_sig,
            index: self.index,
            index_max: self.index_max,
            edid_hash: 0, // XML 中无法获取，需要后续补充
        };
        sk.list_index = self.list_index;
        sk
    }
}

/// 解析 Delphi xTranslator XML 导出文件
///
/// XML 格式说明：
/// ```xml
/// <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
/// <SSTXMLRessources>
///   <Params>
///     <Addon>插件名</Addon>
///     <Source>源语言（如 english）</Source>
///     <Dest>目标语言（如 chinese）</Dest>
///     <Version>格式版本（通常为2）</Version>
///   </Params>
///   <Content>
///     <String List="0" sID="000001">
///       <EDID>Editor ID</EDID>
///       <REC id="0" idMax="0">RECORD:FIELD</REC>
///       <Source>源文本</Source>
///       <Dest>翻译文本</Dest>
///     </String>
///   </Content>
/// </SSTXMLRessources>
/// ```
///
/// 字段说明：
/// - `List`: Strings 文件类型（0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS）
/// - `sID`: 字符串 ID（十六进制，6 位）
/// - `EDID`: Editor ID（可选，用于 T2 匹配）
/// - `REC`: 记录类型和字段（格式：RECORD:FIELD）
/// - `id`: 字符串在 Strings 文件中的索引
/// - `idMax`: 最大索引值（用于验证）
/// - `Source`: 源文本
/// - `Dest`: 翻译文本
///
/// 注意：XML 实体（如 &lt; &amp; &gt;）会被正确解码
pub fn parse_xml_export<R: BufRead>(reader: R) -> Result<(XmlExportParams, Vec<XmlStringEntry>)> {
    let mut xml_reader = Reader::from_reader(reader);
    // 重要：不要开启 trim_text(true)。
    // 原因：它会吞掉纯空白文本事件，进而改变实体解码后的原始空格布局。

    let mut params = XmlExportParams {
        addon: String::new(),       // 插件/模组名称
        source_lang: String::new(), // 源语言
        dest_lang: String::new(),   // 目标语言
        version: 0,                 // XML 格式版本
    };

    let mut entries = Vec::new(); // 解析出的字符串条目
    let mut buf = Vec::new(); // XML 解析缓冲区
    let mut current_element = String::new(); // 当前元素名称
    let mut current_string: Option<XmlStringBuilder> = None; // 正在构建的字符串条目

    loop {
        let event = match xml_reader.read_event_into(&mut buf) {
            Ok(e) => e,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "XML parse error at pos {}: {}",
                    xml_reader.error_position(),
                    e
                ))
            }
        };

        match event {
            Event::Start(e) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match current_element.as_str() {
                    "String" => {
                        // 解析 String 节点属性
                        let mut list_index = 0u8;
                        let mut str_id = 0i32;

                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match key.as_str() {
                                "List" => {
                                    list_index = value.parse().unwrap_or(0);
                                }
                                "sID" => {
                                    str_id = i32::from_str_radix(&value, 16).unwrap_or(0);
                                }
                                _ => {}
                            }
                        }

                        current_string = Some(XmlStringBuilder {
                            list_index,
                            str_id,
                            edid: None,
                            record_sig: [0; 4],
                            field_sig: [0; 4],
                            index: 0,
                            index_max: 0,
                            rec_text: String::new(),
                            source: String::new(),
                            translation: String::new(),
                        });
                    }
                    // String 的子节点：在这里处理 REC 属性
                    "REC" => {
                        // 解析 REC 属性
                        if let Some(ref mut s) = current_string {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let value = String::from_utf8_lossy(&attr.value).to_string();

                                match key.as_str() {
                                    "id" => {
                                        s.index = value.parse().unwrap_or(0);
                                    }
                                    "idMax" => {
                                        s.index_max = value.parse().unwrap_or(0);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    "EDID" | "Source" | "Dest" => {
                        // 这些节点内容在 Text 事件中处理
                    }
                    // Params 区域节点，无需额外处理
                    "Params" | "Addon" | "Version" => {}
                    _ => {}
                }
            }
            Event::Empty(e) => {
                // 处理自闭合标签，如 <REC id="9" idMax="9"/>
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match name.as_str() {
                    "REC" => {
                        // 先解析属性
                        if let Some(ref mut s) = current_string {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let value = String::from_utf8_lossy(&attr.value).to_string();

                                match key.as_str() {
                                    "id" => {
                                        s.index = value.parse().unwrap_or(0);
                                    }
                                    "idMax" => {
                                        s.index_max = value.parse().unwrap_or(0);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(e) => {
                let text = e
                    .decode()
                    .map_err(|err| anyhow::anyhow!("XML text decode error: {}", err))?
                    .to_string();

                // 在 String 节点内：累积全部文本（包括空白）
                // 在 String 节点外：仅保留非空白文本（Params 区域）
                if let Some(ref mut s) = current_string {
                    match current_element.as_str() {
                        "EDID" => {
                            s.edid = if text.is_empty() { None } else { Some(text) };
                        }
                        "REC" => {
                            s.rec_text.push_str(&text);
                        }
                        "Source" => {
                            s.source.push_str(&text);
                        }
                        "Dest" => {
                            s.translation.push_str(&text);
                        }
                        _ => {}
                    }
                } else {
                    // String 节点外：Params 区域
                    // 跳过纯空白文本（缩进）
                    if text.trim().is_empty() {
                        continue;
                    }
                    match current_element.as_str() {
                        "Addon" => {
                            params.addon.push_str(text.trim());
                        }
                        "Source" => {
                            params.source_lang.push_str(text.trim());
                        }
                        "Dest" => {
                            params.dest_lang.push_str(text.trim());
                        }
                        "Version" => {
                            params.version = text.trim().parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
            }
            Event::GeneralRef(e) => {
                // quick-xml 0.38+：XML 实体（如 &lt; &amp; &gt;）会单独上报
                // e.deref() 返回 & 与 ; 之间的内容（如 "lt"、"amp"、"#60"）
                let decoded = decode_xml_entity(&e);

                if let Some(ref mut s) = current_string {
                    match current_element.as_str() {
                        "EDID" => {
                            if let Some(ref mut edid) = s.edid {
                                edid.push_str(&decoded);
                            } else {
                                s.edid = Some(decoded);
                            }
                        }
                        "REC" => {
                            s.rec_text.push_str(&decoded);
                        }
                        "Source" => {
                            s.source.push_str(&decoded);
                        }
                        "Dest" => {
                            s.translation.push_str(&decoded);
                        }
                        _ => {}
                    }
                } else {
                    match current_element.as_str() {
                        "Addon" => {
                            params.addon.push_str(&decoded);
                        }
                        "Source" => {
                            params.source_lang.push_str(&decoded);
                        }
                        "Dest" => {
                            params.dest_lang.push_str(&decoded);
                        }
                        _ => {}
                    }
                }
            }
            Event::End(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "String" {
                    if let Some(s) = current_string.take() {
                        // 解析 REC 文本："RECORD:FIELD"
                        let (record_sig, field_sig) =
                            if let Some((rec, field)) = s.rec_text.split_once(':') {
                                (parse_sig(rec), parse_sig(field))
                            } else {
                                (s.record_sig, s.field_sig)
                            };
                        entries.push(XmlStringEntry {
                            list_index: s.list_index,
                            str_id: s.str_id,
                            edid: s.edid,
                            record_sig,
                            field_sig,
                            index: s.index,
                            index_max: s.index_max,
                            // 与 Delphi 导出行为对齐：写入前做首尾 trim，避免缩进噪声进入正文。
                            source: s.source.trim().to_string(),
                            translation: s.translation.trim().to_string(),
                        });
                    }
                }
                current_element.clear();
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok((params, entries))
}

/// 解码 XML 实体引用（即 & 与 ; 之间的内容）
fn decode_xml_entity(entity: &[u8]) -> String {
    let s = std::str::from_utf8(entity).unwrap_or("");
    match s {
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "amp" => "&".to_string(),
        "apos" => "'".to_string(),
        "quot" => "\"".to_string(),
        _ if s.starts_with('#') => {
            // 字符引用：&#NNN; 或 &#xHH;
            let num_str = &s[1..];
            if let Some(hex) = num_str.strip_prefix('x') {
                u32::from_str_radix(hex, 16)
                    .ok()
                    .and_then(|cp| char::from_u32(cp))
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            } else {
                num_str
                    .parse::<u32>()
                    .ok()
                    .and_then(|cp| char::from_u32(cp))
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            }
        }
        // 未知实体统一丢弃，保持容错，不让解析流程因单个非法实体失败。
        _ => String::new(),
    }
}

/// 解析 4 字节 record/field 签名
fn parse_sig(s: &str) -> HeaderSig {
    let bytes = s.as_bytes();
    let mut sig = [0u8; 4];
    for (i, &b) in bytes.iter().take(4).enumerate() {
        sig[i] = b;
    }
    sig
}

#[derive(Debug)]
struct XmlStringBuilder {
    list_index: u8,
    str_id: i32,
    edid: Option<String>,
    record_sig: HeaderSig,
    field_sig: HeaderSig,
    index: u16,
    index_max: u16,
    rec_text: String,
    source: String,
    translation: String,
}

/// 从文件路径解析 XML
pub fn parse_xml_file(path: &Path) -> Result<(XmlExportParams, Vec<XmlStringEntry>)> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open XML file: {}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    parse_xml_export(reader)
}

/// 按 Delphi xTranslator 兼容格式将 XML 内容写入任意写入器
///
/// 导出传入的所有 `entries`，调用方负责在传入前将数据过滤（
/// 例如通过 `sky_strings_to_xml_entries` 只传入已翻译的条目）。
/// 写入成功后返回 `Ok(())`。
pub fn write_xml_export<W: Write>(
    writer: &mut W,
    params: &XmlExportParams,
    entries: &[XmlStringEntry],
) -> Result<()> {
    // XML 转义辅助函数
    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    writeln!(
        writer,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    )?;
    writeln!(writer, "<SSTXMLRessources>")?;
    writeln!(writer, "  <Params>")?;
    writeln!(writer, "    <Addon>{}</Addon>", escape(&params.addon))?;
    writeln!(
        writer,
        "    <Source>{}</Source>",
        escape(&params.source_lang)
    )?;
    writeln!(writer, "    <Dest>{}</Dest>", escape(&params.dest_lang))?;
    writeln!(writer, "    <Version>{}</Version>", params.version)?;
    writeln!(writer, "  </Params>")?;
    writeln!(writer, "  <Content>")?;

    for entry in entries {
        let rec_text = format!(
            "{}:{}",
            String::from_utf8_lossy(&entry.record_sig),
            String::from_utf8_lossy(&entry.field_sig)
        );
        // sID 使用 6 位十六进制大写，保持与 Delphi 文件习惯一致。
        let sid = format!("{:06X}", entry.str_id);

        writeln!(
            writer,
            r#"    <String List="{}" sID="{}">"#,
            entry.list_index, sid
        )?;

        if let Some(ref edid) = entry.edid {
            writeln!(writer, "      <EDID>{}</EDID>", escape(edid))?;
        } else {
            writeln!(writer, "      <EDID/>")?;
        }

        if entry.index_max > 0 {
            writeln!(
                writer,
                r#"      <REC id="{}" idMax="{}">{}</REC>"#,
                entry.index,
                entry.index_max,
                escape(&rec_text)
            )?;
        } else {
            writeln!(writer, r#"      <REC>{}</REC>"#, escape(&rec_text))?;
        }

        writeln!(writer, "      <Source>{}</Source>", escape(&entry.source))?;
        writeln!(writer, "      <Dest>{}</Dest>", escape(&entry.translation))?;
        writeln!(writer, "    </String>")?;
    }

    writeln!(writer, "  </Content>")?;
    writeln!(writer, "</SSTXMLRessources>")?;

    Ok(())
}

/// 将 XML 导出写入文件
pub fn write_xml_file(
    path: &Path,
    params: &XmlExportParams,
    entries: &[XmlStringEntry],
) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create XML file: {}", path.display()))?;
    write_xml_export(&mut file, params, entries)
}

/// 将 SkyString 列表转换为 XML 导出格式
///
/// Delphi 行为：只导出已翻译的字符串（translation 非空）
/// 这是为了保持 XML 文件简洁，只包含实际翻译内容
///
/// # 参数
/// * `strings` - SkyString 列表（通常来自 AppState.strings）
///
/// # 返回
/// 适合导出为 XML 的条目列表
///
/// # 注意
/// - list_index 从 SkyString.list_index 读取（ESP 解析时已填充）
/// - edid 取 SkyString.edid（ESP 解析期从记录 EDID 字段填充，普通与 VMAD 均覆盖）
pub fn sky_strings_to_xml_entries(strings: &[SkyString]) -> Vec<XmlStringEntry> {
    strings
        .iter()
        .filter(|sk| !sk.translation.is_empty()) // 仅导出已翻译字符串
        .map(|sk| XmlStringEntry {
            list_index: sk.list_index,
            str_id: sk.esp_ptr.str_id,           // 字符串 ID（用于匹配）
            edid: sk.edid.clone(),               // 解析期提取的 Editor ID
            record_sig: sk.esp_ptr.record_sig,   // 记录类型签名
            field_sig: sk.esp_ptr.field_sig,     // 字段签名
            index: sk.esp_ptr.index,             // 字段索引
            index_max: sk.esp_ptr.index_max,     // 字段总数
            source: sk.source.clone(),           // 源文本
            translation: sk.translation.clone(), // 译文
        })
        .collect()
}

/// XML 导出的记录范围，对齐 Delphi `TFormXmlOpt.RadioGroup1` 的 4 档
/// （`TESVT_XMLExportOpts.dfm` + `TESVT_main.pas` 中 `iXMLExportOpt` 的映射）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum XmlExportScope {
    /// 0 - XMLExport_All：候选集内全部条目（Delphi `compareOptEverything`）
    #[default]
    Everything = 0,
    /// 1 - XMLExport_Translated：仅已翻译或已验证（Delphi `compareOptTranslatedAndValidated`）
    TranslatedAndValidated = 1,
    /// 2 - XMLExport_Selection：仅当前选中项（Delphi `compareOptSelection`）
    Selection = 2,
    /// 3 - XMLExport_Diff：源文 != 译文，或带协作 ID（Delphi `compareSourceDestDiffandColab`）
    SourceDestDiff = 3,
}

impl XmlExportScope {
    /// 从 0..3 的 RadioGroup ItemIndex 解析（Delphi `iXMLExportOpt`）。
    pub fn from_radio_index(i: i32) -> Option<Self> {
        match i {
            0 => Some(Self::Everything),
            1 => Some(Self::TranslatedAndValidated),
            2 => Some(Self::Selection),
            3 => Some(Self::SourceDestDiff),
            _ => None,
        }
    }
}

/// XML 导出选项（Delphi `TFormXmlOpt` 的对话框状态）。
#[derive(Debug, Clone, Default)]
pub struct XmlExportOptions {
    /// 记录范围（对应 RadioGroup1.ItemIndex）。
    pub scope: XmlExportScope,
    /// Selection 范围使用的稳定 `u32 id` 集合（Delphi `compareOptSelection` 检查
    /// `treearray[sk.listIndex].Selected[...]`，前端把多选集合映射为 id 集）。
    pub selected_ids: Option<HashSet<u32>>,
    /// 是否导出对话条目的 FUZ 数据（Delphi `chk_exportFuzData`）。当前 FUZ
    /// 元数据尚未在 XML 导出层接通，保留该开关以对齐对话框行为。
    pub export_fuz: bool,
}

/// 判断字符串是否满足 Delphi `tSkyStr.isEmpty`：源文与译文均为空。
fn is_empty_sky(sk: &SkyString) -> bool {
    sk.source.is_empty() && sk.translation.is_empty()
}

/// 判断字符串是否处于 Delphi `tSkyStr.lockedStatus`：
/// `pexNoTrans in sInternalparams`（PEX 脚本无翻译）或
/// `isVMADString in sInternalparams` 且 `lockedTrans in sparams`（isLockedVmad）。
pub fn is_locked_status(sk: &SkyString) -> bool {
    if sk
        .internal_params
        .is_set(SkyStringInternalParams::PEX_NO_TRANS)
    {
        return true;
    }
    sk.internal_params
        .is_set(SkyStringInternalParams::IS_VMAD_STRING)
        && sk.params.is_set(SkyStringParams::LOCKED_TRANS)
}

/// 判断字符串是否命中 Delphi `prepareSSTXML` 的排除条件：
/// 内部标记 `[isDeleted, lowwarning, warning, bigwarning, nTrans]` 任一命中即排除。
pub fn is_prepare_sstxml_excluded(sk: &SkyString) -> bool {
    const EXCLUDED: u64 = SkyStringInternalParams::IS_DELETED
        | SkyStringInternalParams::LOW_WARNING
        | SkyStringInternalParams::WARNING
        | SkyStringInternalParams::BIG_WARNING
        | SkyStringInternalParams::N_TRANS;
    sk.internal_params.0 & EXCLUDED != 0
}

/// 判断字符串是否命中 Delphi `XMLExportbase` 候选集要求：
/// `sparams * [translated, lockedTrans, incompleteTrans, validated] <> []`
/// （即至少有一个“参与导出”的状态位）。
///
/// `params` 应为归一化后的状态位（见 `normalize_export_params`）。
fn has_export_state(params: SkyStringParams) -> bool {
    const EXPORT_STATES: u8 = SkyStringParams::TRANSLATED
        | SkyStringParams::LOCKED_TRANS
        | SkyStringParams::INCOMPLETE_TRANS
        | SkyStringParams::VALIDATED;
    params.0 & EXPORT_STATES != 0
}

/// 按 Delphi `XMLExportbase` 的 scope comparator 判断 `sk` 是否导出。
///
/// `params` 应为候选集归一化后的状态位（见 `normalize_export_params`）；
/// `prepareSSTXML` 对 lockedTrans 条目清掉 translated/incomplete/validated 后
/// 才应用 `fProc`。Selection 无选中集合时按“无选中项”处理。
pub fn xml_scope_matches(sk: &SkyString, params: SkyStringParams, opts: &XmlExportOptions) -> bool {
    match opts.scope {
        XmlExportScope::Everything => true,
        XmlExportScope::TranslatedAndValidated => {
            params.is_set(SkyStringParams::TRANSLATED)
                || params.is_set(SkyStringParams::VALIDATED)
        }
        XmlExportScope::Selection => opts
            .selected_ids
            .as_ref()
            .is_some_and(|ids| ids.contains(&sk.id)),
        XmlExportScope::SourceDestDiff => {
            // compareSourceDestDiffandColab: (colabId <> 0) or (hash <> hashTrans)
            sk.colab_id != 0 || sk.hash != sk.hash_trans
        }
    }
}

/// Delphi `prepareSSTXML` 的 lockedTrans 归一化：lockedTrans 条目清掉
/// translated / incompleteTrans / validated（只保留锁定状态）。
pub fn normalize_export_params(params: SkyStringParams) -> SkyStringParams {
    let mut p = params;
    if p.is_set(SkyStringParams::LOCKED_TRANS) {
        p.set(SkyStringParams::TRANSLATED, false);
        p.set(SkyStringParams::INCOMPLETE_TRANS, false);
        p.set(SkyStringParams::VALIDATED, false);
    }
    p
}

/// 收集要导出到 XML 的条目（Delphi `XMLExportbase` 等价语义）。
///
/// 分两步：
/// 1. `prepareSSTXML` 候选集：排除空串 / `lockedStatus`（pexNoTrans、locked
///    VMAD）/ 内部删除与警告标记 / 没有任何导出状态位的字符串；lockedTrans
///    条目按 Delphi 归一化（清掉 translated/incomplete/validated）；
/// 2. 在候选集上应用 scope comparator（`compareOptEverything` /
///    `compareOptTranslatedAndValidated` / `compareOptSelection` /
///    `compareSourceDestDiffandColab`）。
///
/// 与 `sky_strings_to_xml_entries`（固定“已有译文”的快速导出）不同，本函数
/// 供交互式 XML Export 对话框使用，逐项对齐原版选项。
pub fn collect_xml_export_entries(
    strings: &[SkyString],
    opts: &XmlExportOptions,
) -> Vec<XmlStringEntry> {
    strings
        .iter()
        // prepareSSTXML 候选集
        .filter(|sk| !is_empty_sky(sk))
        .filter(|sk| !is_locked_status(sk))
        .filter(|sk| !is_prepare_sstxml_excluded(sk))
        .filter(|sk| has_export_state(normalize_export_params(sk.params)))
        // scope comparator（在归一化状态位上判定）
        .filter(|sk| xml_scope_matches(sk, normalize_export_params(sk.params), opts))
        .map(|sk| XmlStringEntry {
            list_index: sk.list_index,
            str_id: sk.esp_ptr.str_id,
            edid: sk.edid.clone(),
            record_sig: sk.esp_ptr.record_sig,
            field_sig: sk.esp_ptr.field_sig,
            index: sk.esp_ptr.index,
            index_max: sk.esp_ptr.index_max,
            source: sk.source.clone(),
            translation: sk.translation.clone(),
        })
        .collect()
}

/// 将 XML 导入条目应用到现有的 SkyString 列表
///
/// 使用增强多层级匹配策略：
/// 1. 精确三元组 (str_id, record_sig, field_sig)
/// 2. EDID 哈希匹配（跨版本稳定）
/// 3. 规范化文本哈希匹配
/// 4. 词汇重叠匹配（Jaccard 相似度）
///
/// # 参数
/// * `strings` - 可变的 SkyString 切片（通常来自 AppState.strings）
/// * `xml_entries` - 从 XML 解析出的条目列表
///
/// # 返回
/// 包含各层级匹配统计的 `MatchResult`
pub fn import_xml_to_sky_strings(
    strings: &mut [SkyString],
    xml_entries: &[XmlStringEntry],
) -> crate::matching::MatchResult {
    crate::matching::apply_xml_dictionary_entries(strings, xml_entries)
}

/// 带应用策略的 XML 导入：processor 的 ImportXml 按与 SST 相同的
/// scope / match_mode comparator 家族执行，未匹配条目不保留 OLD_DATA。
pub fn import_xml_to_sky_strings_with_policy(
    strings: &mut [SkyString],
    xml_entries: &[XmlStringEntry],
    policy: crate::matching::ApplyPolicy,
) -> crate::matching::MatchResult {
    crate::matching::apply_xml_dictionary_entries_with_policy(strings, xml_entries, policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<SSTXMLRessources>
  <Params>
    <Addon>test</Addon>
    <Source>english</Source>
    <Dest>chinese</Dest>
    <Version>2</Version>
  </Params>
  <Content>
    <String List="0" sID="000001">
      <EDID>TestEDID</EDID>
      <REC>LCTN:FULL</REC>
      <Source>Hello</Source>
      <Dest>你好</Dest>
    </String>
  </Content>
</SSTXMLRessources>"#;

        let (params, entries) = parse_xml_export(Cursor::new(xml)).unwrap();
        assert_eq!(params.addon, "test");
        assert_eq!(params.source_lang, "english");
        assert_eq!(params.dest_lang, "chinese");
        assert_eq!(params.version, 2);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].str_id, 1);
        assert_eq!(entries[0].source, "Hello");
        assert_eq!(entries[0].translation, "你好");
    }

    #[test]
    fn test_parse_indexed_rec() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<SSTXMLRessources>
  <Params><Addon>test</Addon><Source>en</Source><Dest>zh</Dest><Version>2</Version></Params>
  <Content>
    <String List="0" sID="000008">
      <EDID>TestQuest</EDID>
      <REC id="9" idMax="9">QUST:NNAM</REC>
      <Source>Retrieve the sword</Source>
      <Dest>取回剑</Dest>
    </String>
  </Content>
</SSTXMLRessources>"#;

        let (_, entries) = parse_xml_export(Cursor::new(xml)).unwrap();
        assert_eq!(entries[0].index, 9);
        assert_eq!(entries[0].index_max, 9);
    }

    #[test]
    fn test_parse_sig() {
        assert_eq!(parse_sig("LCTN"), [b'L', b'C', b'T', b'N']);
        assert_eq!(parse_sig("FULL"), [b'F', b'U', b'L', b'L']);
    }

    #[test]
    fn test_write_and_parse_roundtrip() {
        let params = XmlExportParams {
            addon: "Skyrim".to_string(),
            source_lang: "english".to_string(),
            dest_lang: "chinese".to_string(),
            version: 2,
        };

        let entries = vec![
            XmlStringEntry {
                list_index: 0,
                str_id: 1,
                edid: Some("TestEDID".to_string()),
                record_sig: *b"LCTN",
                field_sig: *b"FULL",
                index: 0,
                index_max: 0,
                source: "Hello World".to_string(),
                translation: "你好世界".to_string(),
            },
            XmlStringEntry {
                list_index: 1,
                str_id: 8,
                edid: None,
                record_sig: *b"QUST",
                field_sig: *b"NNAM",
                index: 9,
                index_max: 9,
                source: "Retrieve the sword".to_string(),
                translation: "取回剑".to_string(),
            },
        ];

        let mut buf = Vec::new();
        write_xml_export(&mut buf, &params, &entries).unwrap();

        let xml_str = String::from_utf8(buf).unwrap();

        // 回读并校验
        let (parsed_params, parsed_entries) = parse_xml_export(Cursor::new(xml_str)).unwrap();

        assert_eq!(parsed_params.addon, "Skyrim");
        assert_eq!(parsed_params.source_lang, "english");
        assert_eq!(parsed_params.dest_lang, "chinese");
        assert_eq!(parsed_params.version, 2);

        assert_eq!(parsed_entries.len(), 2);

        assert_eq!(parsed_entries[0].str_id, 1);
        assert_eq!(parsed_entries[0].edid, Some("TestEDID".to_string()));
        assert_eq!(parsed_entries[0].source, "Hello World");
        assert_eq!(parsed_entries[0].translation, "你好世界");

        assert_eq!(parsed_entries[1].str_id, 8);
        assert_eq!(parsed_entries[1].edid, None);
        assert_eq!(parsed_entries[1].index, 9);
        assert_eq!(parsed_entries[1].index_max, 9);
        assert_eq!(parsed_entries[1].source, "Retrieve the sword");
        assert_eq!(parsed_entries[1].translation, "取回剑");
    }

    #[test]
    fn test_xml_escape() {
        let params = XmlExportParams {
            addon: "Test & Mod".to_string(),
            source_lang: "en".to_string(),
            dest_lang: "zh".to_string(),
            version: 2,
        };

        let entries = vec![XmlStringEntry {
            list_index: 0,
            str_id: 1,
            edid: None,
            record_sig: *b"INFO",
            field_sig: *b"NAM1",
            index: 0,
            index_max: 0,
            source: "A < B & C > D".to_string(),
            translation: "X < Y & Z > W".to_string(),
        }];

        let mut buf = Vec::new();
        write_xml_export(&mut buf, &params, &entries).unwrap();

        let xml_str = String::from_utf8(buf).unwrap();
        assert!(xml_str.contains("A &lt; B &amp; C &gt; D"));
        assert!(xml_str.contains("X &lt; Y &amp; Z &gt; W"));
        assert!(xml_str.contains("Test &amp; Mod"));

        // 回读并校验
        let (_, parsed) = parse_xml_export(Cursor::new(xml_str)).unwrap();
        assert_eq!(parsed[0].source, "A < B & C > D");
        assert_eq!(parsed[0].translation, "X < Y & Z > W");
    }

    #[test]
    fn test_import_xml_to_sky_strings() {
        let mut strings = vec![
            SkyString::new(0, "Hello".to_string(), String::new(), *b"HELO", *b"TXT "),
            SkyString::new(1, "World".to_string(), String::new(), *b"WORL", *b"TXT "),
        ];
        strings[0].esp_ptr.str_id = 1;
        strings[0].esp_ptr.record_sig = *b"LCTN";
        strings[0].esp_ptr.field_sig = *b"FULL";
        strings[1].esp_ptr.str_id = 8;
        strings[1].esp_ptr.record_sig = *b"QUST";
        strings[1].esp_ptr.field_sig = *b"NNAM";

        let xml_entries = vec![
            XmlStringEntry {
                list_index: 0,
                str_id: 1,
                edid: None,
                record_sig: *b"LCTN",
                field_sig: *b"FULL",
                index: 0,
                index_max: 0,
                source: "Hello".to_string(),
                translation: "你好".to_string(),
            },
            XmlStringEntry {
                list_index: 0,
                str_id: 99,
                edid: None,
                record_sig: *b"NPC_",
                field_sig: *b"FULL",
                index: 0,
                index_max: 0,
                source: "Unknown".to_string(),
                translation: "未知".to_string(),
            },
        ];

        let result = import_xml_to_sky_strings(&mut strings, &xml_entries);

        assert_eq!(result.tier_exact, 1);
        assert_eq!(result.total_matched(), 1);
        assert_eq!(result.unmatched, 1);
        assert_eq!(strings[0].translation, "你好");
        assert!(strings[0].params.is_translated());
        assert!(strings[1].translation.is_empty());
    }

    #[test]
    fn test_sky_strings_to_xml_entries_filters_empty() {
        let mut strings = vec![
            SkyString::new(
                0,
                "Hello".to_string(),
                "你好".to_string(),
                *b"HELO",
                *b"TXT ",
            ),
            SkyString::new(1, "World".to_string(), String::new(), *b"WORL", *b"TXT "), // 空翻译
        ];
        strings[0].esp_ptr.str_id = 1;
        strings[0].esp_ptr.record_sig = *b"LCTN";
        strings[0].esp_ptr.field_sig = *b"FULL";
        strings[1].esp_ptr.str_id = 2;
        strings[1].esp_ptr.record_sig = *b"QUST";
        strings[1].esp_ptr.field_sig = *b"NNAM";

        let entries = sky_strings_to_xml_entries(&strings);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "Hello");
        assert_eq!(entries[0].translation, "你好");
    }

    /// 构造一个带完整导出相关状态的 SkyString。
    fn make_export_string(
        id: u32,
        source: &str,
        translation: &str,
        set_state: impl FnOnce(&mut SkyString),
    ) -> SkyString {
        let mut sk = SkyString::new(
            id,
            source.to_string(),
            translation.to_string(),
            *b"LCTN",
            *b"FULL",
        );
        sk.esp_ptr.str_id = id as i32 + 100;
        sk.edid = Some(format!("EDID{id}"));
        sk.colab_id = if id % 2 == 0 { 0 } else { 7 };
        sk.hash = crate::types::esp_pointer::string_hash(source);
        sk.hash_trans = crate::types::esp_pointer::string_hash(translation);
        set_state(&mut sk);
        sk
    }

    fn translated(sk: &mut SkyString) {
        sk.params.set(SkyStringParams::TRANSLATED, true);
    }

    fn validated(sk: &mut SkyString) {
        sk.params.set(SkyStringParams::VALIDATED, true);
    }

    fn locked(sk: &mut SkyString) {
        sk.params.set(SkyStringParams::LOCKED_TRANS, true);
        sk.params.set(SkyStringParams::TRANSLATED, true);
    }

    fn no_trans(sk: &mut SkyString) {
        sk.internal_params
            .set(SkyStringInternalParams::PEX_NO_TRANS, true);
    }

    #[test]
    fn test_xml_export_scope_everything() {
        // 已翻译 + 空译文（无状态位）→ 候选集仅已翻译项
        let strings = vec![
            make_export_string(0, "Alpha", "A-zh", translated),
            make_export_string(1, "Beta", "B-zh", validated),
            make_export_string(2, "Gamma", "", |_| {}),
        ];
        let opts = XmlExportOptions::default(); // Everything
        let entries = collect_xml_export_entries(&strings, &opts);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].edid.as_deref(), Some("EDID0"));
    }

    #[test]
    fn test_xml_export_scope_translated_validated() {
        let strings = vec![
            make_export_string(0, "Alpha", "A-zh", translated),
            make_export_string(1, "Beta", "B-zh", validated),
            make_export_string(2, "Gamma", "G-zh", |sk| {
                sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
            }),
        ];
        let opts = XmlExportOptions {
            scope: XmlExportScope::TranslatedAndValidated,
            ..Default::default()
        };
        let entries = collect_xml_export_entries(&strings, &opts);
        assert_eq!(entries.len(), 2); // incomplete 排除
    }

    #[test]
    fn test_xml_export_scope_selection_uses_stable_ids() {
        let strings = vec![
            make_export_string(10, "Alpha", "A-zh", translated),
            make_export_string(20, "Beta", "B-zh", validated),
        ];
        let opts = XmlExportOptions {
            scope: XmlExportScope::Selection,
            selected_ids: Some(HashSet::from([20])),
            ..Default::default()
        };
        let entries = collect_xml_export_entries(&strings, &opts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "Beta");
    }

    #[test]
    fn test_xml_export_scope_selection_empty_set_exports_none() {
        let strings = vec![make_export_string(10, "Alpha", "A-zh", translated)];
        let opts = XmlExportOptions {
            scope: XmlExportScope::Selection,
            selected_ids: Some(HashSet::new()),
            ..Default::default()
        };
        assert!(collect_xml_export_entries(&strings, &opts).is_empty());
    }

    #[test]
    fn test_xml_export_scope_diff_colab_or_hash_mismatch() {
        // colab_id != 0 → diff
        let strings = vec![
            make_export_string(0, "Same", "Same-zh", translated), // colab 0, hash differs
            make_export_string(1, "Colab", "Colab-zh", translated), // colab 7
            make_export_string(2, "Identical", "Identical", translated), // colab 0, hash same
        ];
        let opts = XmlExportOptions {
            scope: XmlExportScope::SourceDestDiff,
            ..Default::default()
        };
        let entries = collect_xml_export_entries(&strings, &opts);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.source != "Identical"));
    }

    #[test]
    fn test_xml_export_excludes_locked_status_and_warnings() {
        let strings = vec![
            make_export_string(0, "Normal", "N-zh", translated),
            make_export_string(1, "PexNoTrans", "", no_trans),
            make_export_string(2, "Warning", "W-zh", |sk| {
                sk.params.set(SkyStringParams::TRANSLATED, true);
                sk.internal_params
                    .set(SkyStringInternalParams::LOW_WARNING, true);
            }),
            make_export_string(3, "Deleted", "D-zh", |sk| {
                sk.params.set(SkyStringParams::TRANSLATED, true);
                sk.internal_params
                    .set(SkyStringInternalParams::IS_DELETED, true);
            }),
        ];
        let entries = collect_xml_export_entries(&strings, &XmlExportOptions::default());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "Normal");
    }

    #[test]
    fn test_xml_export_normalizes_locked_trans_state() {
        // lockedTrans 条目被归一化（清 translated），但 locked 本身仍在候选集 → 需再判 scope
        let strings = vec![
            make_export_string(0, "Locked", "L-zh", locked),
            make_export_string(1, "Normal", "N-zh", translated),
        ];
        let opts = XmlExportOptions {
            scope: XmlExportScope::TranslatedAndValidated,
            ..Default::default()
        };
        let entries = collect_xml_export_entries(&strings, &opts);
        // locked 条目归一化后无 translated → 不满足 Translated scope
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "Normal");
    }

    #[test]
    fn test_xml_export_edid_preserved_through_collect() {
        let strings = vec![make_export_string(0, "Alpha", "A-zh", translated)];
        let entries = collect_xml_export_entries(&strings, &XmlExportOptions::default());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].edid.as_deref(), Some("EDID0"));
        assert_eq!(entries[0].str_id, 100);
    }
}
