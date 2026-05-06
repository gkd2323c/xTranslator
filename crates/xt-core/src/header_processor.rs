//! Header Processor — rule-based text transformation pipeline.
//!
//! Corresponds to Delphi TESVT_FormData.pas + TESVT_Templates.pas.

use serde::{Deserialize, Serialize};

// ── DTOs for IPC ──────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeaderRuleDto {
    pub index: usize,
    pub header: String,
    pub r_sig: String,
    pub f_sig: String,
    pub enabled: bool,
    pub in_edid: Vec<String>,
    pub ex_edid: Vec<String>,
    pub no_kw: bool,
    pub any_kw: bool,
    pub has_component: bool,
    pub include_keywords: Vec<HeaderKeywordDto>,
    pub pre_process: bool,
    pub full_replace: bool,
    pub include_or: bool,
    pub is_fallback: bool,
    pub regex: Option<String>,
    pub tag_id: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeaderKeywordDto {
    pub kw_type: String,
    pub name: String,
    pub form_id: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeaderApplyResult {
    pub total_rules: usize,
    pub enabled_rules: usize,
    pub strings_matched: u32,
}

// ── Core Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderKeyword {
    pub kw_type: String,
    pub name: String,
    pub form_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderRule {
    pub header: String,
    pub r_sig: String,
    pub f_sig: String,
    pub enabled: bool,
    pub in_edid: Vec<String>,
    pub ex_edid: Vec<String>,
    pub no_kw: bool,
    pub any_kw: bool,
    pub has_component: bool,
    pub include_keywords: Vec<HeaderKeyword>,
    pub pre_process: bool,
    pub full_replace: bool,
    pub include_or: bool,
    pub is_fallback: bool,
    pub regex: Option<String>,
    pub tag_id: i32,
}

impl Default for HeaderRule {
    fn default() -> Self {
        Self {
            header: String::new(), r_sig: String::new(), f_sig: String::new(),
            enabled: true, in_edid: vec![], ex_edid: vec![],
            no_kw: false, any_kw: false, has_component: false,
            include_keywords: vec![], pre_process: false, full_replace: false,
            include_or: false, is_fallback: false, regex: None, tag_id: 0,
        }
    }
}

impl From<&HeaderRule> for HeaderRuleDto {
    fn from(r: &HeaderRule) -> Self {
        Self {
            index: 0, header: r.header.clone(), r_sig: r.r_sig.clone(),
            f_sig: r.f_sig.clone(), enabled: r.enabled,
            in_edid: r.in_edid.clone(), ex_edid: r.ex_edid.clone(),
            no_kw: r.no_kw, any_kw: r.any_kw, has_component: r.has_component,
            include_keywords: r.include_keywords.iter().map(|kw| HeaderKeywordDto {
                kw_type: kw.kw_type.clone(), name: kw.name.clone(), form_id: kw.form_id,
            }).collect(),
            pre_process: r.pre_process, full_replace: r.full_replace,
            include_or: r.include_or, is_fallback: r.is_fallback,
            regex: r.regex.clone(), tag_id: r.tag_id,
        }
    }
}

impl HeaderRule {
    pub fn matches_string(&self, record_sig: &str, field_sig: &str, edid: &str, form_id: u32, keywords: &[u32]) -> bool {
        if !self.r_sig.is_empty() && !self.r_sig.eq_ignore_ascii_case(record_sig) { return false; }
        if !self.f_sig.is_empty() && !self.f_sig.eq_ignore_ascii_case(field_sig) { return false; }
        if !self.in_edid.is_empty() {
            let edid_lower = edid.to_lowercase();
            let matched = self.in_edid.iter().any(|pat| edid_lower.contains(&pat.to_lowercase()));
            if self.no_kw { if matched { return false; } } else { if !matched { return false; } }
        }
        if !self.ex_edid.is_empty() {
            let edid_lower = edid.to_lowercase();
            if self.ex_edid.iter().any(|pat| edid_lower.contains(&pat.to_lowercase())) { return false; }
        }
        if !self.include_keywords.is_empty() {
            let kw_match = if self.any_kw {
                self.include_keywords.iter().any(|kw| match kw.kw_type.as_str() {
                    "form" => kw.form_id == form_id, "kwd_" => keywords.contains(&kw.form_id), _ => false,
                })
            } else {
                self.include_keywords.iter().all(|kw| match kw.kw_type.as_str() {
                    "form" => kw.form_id == form_id, "kwd_" => keywords.contains(&kw.form_id), _ => false,
                })
            };
            if self.no_kw { if kw_match { return false; } } else { if !kw_match { return false; } }
        }
        true
    }

    pub fn apply(&self, translation: &str, _source: &str) -> String {
        if self.full_replace { return self.header.clone(); }
        if self.header.is_empty() { return translation.to_string(); }
        format!("{} {}", self.header, translation)
    }
}

// ── Rule Set ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderRuleSet {
    pub rules: Vec<HeaderRule>,
    pub game_id: String,
}

impl HeaderRuleSet {
    pub fn new() -> Self {
        Self { rules: Vec::new(), game_id: String::new() }
    }

    pub fn from_ini_text(text: &str) -> Self {
        let mut set = Self::new();
        let mut current: Option<HeaderRule> = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('\u{FEFF}') { continue; }
            if line == "[StartRule]" { current = Some(HeaderRule::default()); }
            else if line == "[EndRule]" {
                if let Some(rule) = current.take() { set.rules.push(rule); }
            } else if let Some(ref mut rule) = current {
                if let Some((key, value)) = line.find('=').map(|p| (line[..p].trim().to_string(), line[p+1..].trim())) {
                    match key.as_str() {
                        "Header" => rule.header = value.to_string(),
                        "rSig" => rule.r_sig = value.to_string(),
                        "fSig" => rule.f_sig = value.to_string(),
                        "enabled" => rule.enabled = value.parse::<i32>().unwrap_or(0) != 0,
                        "inEdid" => rule.in_edid = value.split('|').map(|s| s.to_string()).collect(),
                        "exEdid" => rule.ex_edid = value.split('|').map(|s| s.to_string()).collect(),
                        "noKW" => rule.no_kw = value != "0",
                        "anyKW" => rule.any_kw = value != "0",
                        "hasCompo" => rule.has_component = value != "0",
                        "preProcess" => rule.pre_process = value != "0",
                        "fullReplace" => rule.full_replace = value != "0",
                        "includeOr" => rule.include_or = value != "0",
                        "isFallback" => rule.is_fallback = value != "0",
                        "RegEx" => rule.regex = Some(value.to_string()),
                        "tagID" => rule.tag_id = value.parse().unwrap_or(0),
                        k if k.starts_with("Include_") => {
                            let parts: Vec<&str> = value.split('|').collect();
                            if parts.len() >= 3 {
                                rule.include_keywords.push(HeaderKeyword {
                                    kw_type: parts[0].to_string(), name: parts[1].to_string(),
                                    form_id: u32::from_str_radix(parts[2], 16).unwrap_or(0),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        set
    }

    pub fn to_ini_text(&self) -> String {
        let mut out = String::new();
        for rule in &self.rules {
            out.push_str("[StartRule]\n");
            if !rule.header.is_empty() { out.push_str(&format!("\tHeader={}\n", rule.header)); }
            out.push_str(&format!("\trSig={}\n\tfSig={}\n\tenabled={}\n",
                rule.r_sig, rule.f_sig, if rule.enabled { -1 } else { 0 }));
            if !rule.in_edid.is_empty() { out.push_str(&format!("\tinEdid={}\n", rule.in_edid.join("|"))); }
            if !rule.ex_edid.is_empty() { out.push_str(&format!("\texEdid={}\n", rule.ex_edid.join("|"))); }
            out.push_str(&format!("\tnoKW={}\n\tanyKW={}\n\thasCompo={}\n",
                rule.no_kw as i32, rule.any_kw as i32, rule.has_component as i32));
            for (i, kw) in rule.include_keywords.iter().enumerate() {
                out.push_str(&format!("\tInclude_{}={}|{}|{:08X}\n", i, kw.kw_type, kw.name, kw.form_id));
            }
            if rule.pre_process { out.push_str("\tpreProcess=1\n"); }
            if rule.full_replace { out.push_str("\tfullReplace=1\n"); }
            out.push_str("[EndRule]\n\n");
        }
        out
    }

    pub fn apply_rules(&self, record_sig: &str, field_sig: &str, edid: &str, form_id: u32, keywords: &[u32], translation: &str, source: &str) -> Option<String> {
        for rule in &self.rules {
            if !rule.enabled { continue; }
            if rule.matches_string(record_sig, field_sig, edid, form_id, keywords) {
                return Some(rule.apply(translation, source));
            }
        }
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let text = "[StartRule]\n\tHeader=[$]\n\trSig=MISC\n\tfSig=FULL\n\tenabled=-1\n[EndRule]";
        let set = HeaderRuleSet::from_ini_text(text);
        assert_eq!(set.rules.len(), 1);
        assert_eq!(set.rules[0].header, "[$]");
        assert_eq!(set.rules[0].r_sig, "MISC");
        assert!(set.rules[0].enabled);
    }

    #[test]
    fn test_parse_keywords() {
        let text = "[StartRule]\n\trSig=BOOK\n\tfSig=FULL\n\tenabled=-1\n\tInclude_0=form|TestBook|00001234\n[EndRule]";
        let set = HeaderRuleSet::from_ini_text(text);
        assert_eq!(set.rules[0].include_keywords.len(), 1);
        assert_eq!(set.rules[0].include_keywords[0].form_id, 0x1234);
    }

    #[test]
    fn test_roundtrip() {
        let text = "[StartRule]\n\tHeader=[$]\n\trSig=MISC\n\tfSig=FULL\n\tenabled=-1\n\tInclude_0=form|TestBook|00001234\n[EndRule]\n";
        let set = HeaderRuleSet::from_ini_text(text);
        let output = set.to_ini_text();
        let set2 = HeaderRuleSet::from_ini_text(&output);
        assert_eq!(set2.rules.len(), 1);
    }

    #[test]
    fn test_rule_match_sig() {
        let rule = HeaderRule { r_sig: "BOOK".into(), f_sig: "FULL".into(), ..Default::default() };
        assert!(rule.matches_string("BOOK", "FULL", "", 0, &[]));
        assert!(!rule.matches_string("NPC_", "FULL", "", 0, &[]));
    }

    #[test]
    fn test_apply() {
        let rule = HeaderRule { header: "[$]".into(), full_replace: true, ..Default::default() };
        assert_eq!(rule.apply("Hello", ""), "[$]");
    }

    #[test]
    fn test_apply_prepend() {
        let rule = HeaderRule { header: "(Lock)".into(), ..Default::default() };
        assert_eq!(rule.apply("Chest", ""), "(Lock) Chest");
    }
}
