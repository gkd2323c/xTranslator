//! 对话 HTML 导出
//!
//! 将 DIAL（对话主题）和 INFO（对话回复）构成的对话树导出为 HTML，
//! 用于审校和打印。
//!
//! 结构：
//! - DIAL:FULL → 对话主题名称
//! - INFO:RNAM → 回复提示文本
//! - INFO:NAM1 → 回复内容文本
//! - INFO → parent_form_id → DIAL.form_id

use crate::types::sky_string::SkyString;

/// 对话导出一条记录
#[derive(Debug, Clone)]
pub struct DialEntry {
    /// 对话主题名称 (DIAL:FULL)
    pub topic: String,
    /// 主题译文
    pub topic_trans: String,
    /// DIAL 的 FormID
    pub dial_form_id: u32,
    /// 该主题下的回复列表
    pub responses: Vec<InfoEntry>,
}

/// 单条回复
#[derive(Debug, Clone)]
pub struct InfoEntry {
    /// 回复文本 (INFO:NAM1)
    pub response: String,
    /// 回复译文
    pub response_trans: String,
    /// 提示文本 (INFO:RNAM)，可选
    pub prompt: Option<String>,
    /// INFO 的 FormID
    pub info_form_id: u32,
}

/// 从 SkyString 列表构建对话树
pub fn build_dial_tree(strings: &[SkyString]) -> Vec<DialEntry> {
    use std::collections::HashMap;

    // DIAL FormID → 主题名/译文
    let mut dial_map: HashMap<u32, (String, String)> = HashMap::new();
    // DIAL FormID → Vec<InfoEntry>
    let mut info_map: HashMap<u32, Vec<InfoEntry>> = HashMap::new();
    // INFO FormID → (prompt text, response text, response trans, dial_form_id)
    let mut info_pending: HashMap<u32, (Option<String>, Option<String>, Option<String>, u32)> =
        HashMap::new();

    for s in strings {
        match &s.record_sig {
            b"DIAL" => {
                dial_map.insert(s.esp_ptr.form_id, (s.source.clone(), s.translation.clone()));
            }
            b"INFO" => {
                let form_id = s.esp_ptr.form_id;
                let parent = s.parent_form_id;
                let entry = info_pending
                    .entry(form_id)
                    .or_insert((None, None, None, parent));

                match &s.field_sig {
                    b"RNAM" => entry.0 = Some(s.source.clone()),
                    b"NAM1" => {
                        entry.1 = Some(s.source.clone());
                        entry.2 = Some(s.translation.clone());
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Assemble INFO entries into DIAL topics
    for (info_form_id, (prompt, response, response_trans, parent)) in info_pending {
        if let Some(response) = response {
            let entry = InfoEntry {
                response,
                response_trans: response_trans.unwrap_or_default(),
                prompt,
                info_form_id,
            };
            info_map.entry(parent).or_default().push(entry);
        }
    }

    // Build final dial tree
    let mut tree: Vec<DialEntry> = dial_map
        .into_iter()
        .map(|(form_id, (topic, topic_trans))| {
            let responses = info_map.remove(&form_id).unwrap_or_default();
            DialEntry {
                topic,
                topic_trans,
                dial_form_id: form_id,
                responses,
            }
        })
        .collect();

    // Sort by topic name
    tree.sort_by(|a, b| a.topic.cmp(&b.topic));

    tree
}

/// 将对话树导出为 HTML 字符串
pub fn dial_tree_to_html(tree: &[DialEntry], title: &str) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    html.push_str(
        r#"<style>
body { font-family: 'Segoe UI', system-ui, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; background: #1e1e2e; color: #cdd6f4; }
h1 { color: #cba6f7; border-bottom: 2px solid #45475a; padding-bottom: 8px; }
.dial-topic { margin: 16px 0 4px 0; color: #f5c2e7; font-size: 1.1em; }
.dial-topic small { color: #6c7086; font-size: 0.8em; margin-left: 8px; }
.info-response { margin: 2px 0 2px 24px; padding: 4px 8px; border-left: 3px solid #45475a; }
.info-response .response { color: #a6e3a1; }
.info-response .trans { color: #89b4fa; margin-left: 8px; }
.info-response .prompt { color: #6c7086; font-size: 0.85em; }
.no-response { color: #6c7086; font-style: italic; margin-left: 24px; }
.stats { color: #6c7086; font-size: 0.9em; margin-bottom: 16px; }
</style>
</head>
<body>
"#,
    );

    html.push_str(&format!("<h1>{}</h1>\n", escape_html(title)));

    let total_dials = tree.len();
    let total_responses: usize = tree.iter().map(|d| d.responses.len()).sum();
    html.push_str(&format!(
        "<p class=\"stats\">{} 个对话主题，{} 条回复</p>\n",
        total_dials, total_responses
    ));

    for dial in tree {
        html.push_str(&format!(
            "<div class=\"dial-topic\">{} <small>[{:08X}]</small></div>\n",
            escape_html(&dial.topic),
            dial.dial_form_id
        ));
        if !dial.topic_trans.is_empty() {
            html.push_str(&format!(
                "<div class=\"info-response\"><span class=\"trans\">→ {}</span></div>\n",
                escape_html(&dial.topic_trans)
            ));
        }

        if dial.responses.is_empty() {
            html.push_str("<div class=\"no-response\">（无回复）</div>\n");
        } else {
            for info in &dial.responses {
                if let Some(ref prompt) = info.prompt {
                    html.push_str(&format!(
                        "<div class=\"info-response\"><span class=\"prompt\">[{}]</span> ",
                        escape_html(prompt)
                    ));
                } else {
                    html.push_str("<div class=\"info-response\">");
                }
                html.push_str(&format!(
                    "<span class=\"response\">{}</span>",
                    escape_html(&info.response)
                ));
                if !info.response_trans.is_empty() {
                    html.push_str(&format!(
                        " <span class=\"trans\">→ {}</span>",
                        escape_html(&info.response_trans)
                    ));
                }
                html.push_str("</div>\n");
            }
        }
    }

    html.push_str("</body>\n</html>");
    html
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sky_string(
        source: &str,
        trans: &str,
        record_sig: &[u8; 4],
        field_sig: &[u8; 4],
        form_id: u32,
        parent_form_id: u32,
    ) -> SkyString {
        let mut sk = SkyString::new(0, source.into(), trans.into(), *record_sig, *field_sig);
        sk.esp_ptr.form_id = form_id;
        sk.parent_form_id = parent_form_id;
        sk
    }

    #[test]
    fn test_build_dial_tree_simple() {
        let strings = vec![
            make_sky_string("Topic 1", "", b"DIAL", b"FULL", 0x100, 0),
            make_sky_string("Hello there", "你好", b"INFO", b"NAM1", 0x200, 0x100),
            make_sky_string("Goodbye", "再见", b"INFO", b"NAM1", 0x201, 0x100),
        ];

        let tree = build_dial_tree(&strings);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].topic, "Topic 1");
        assert_eq!(tree[0].responses.len(), 2);
    }

    #[test]
    fn test_html_output() {
        let strings = vec![
            make_sky_string("Greetings", "问候", b"DIAL", b"FULL", 0x100, 0),
            make_sky_string("How are you?", "你好吗", b"INFO", b"NAM1", 0x200, 0x100),
        ];

        let tree = build_dial_tree(&strings);
        let html = dial_tree_to_html(&tree, "Test Dialog");
        assert!(html.contains("Greetings"));
        assert!(html.contains("How are you"));
        assert!(html.contains("你好吗"));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_empty_input() {
        let tree = build_dial_tree(&[]);
        assert!(tree.is_empty());
        let html = dial_tree_to_html(&tree, "Empty");
        assert!(html.contains("0 个对话主题"));
    }
}
