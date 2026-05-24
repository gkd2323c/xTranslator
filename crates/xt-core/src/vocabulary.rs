//! Vocabulary loader — builds source→translation pairs from game Strings files.
//!
//! Parses `vocabulary.txt` to get a list of Strings file base names,
//! then loads source-language and target-language files, matches them by
//! str_id, and produces `(source, translation)` pairs for use in
//! heuristic search and auto-translation suggestions.
//!
//! This mirrors the Delphi xTranslator's "vocabulary" feature:
//! the `vocabulary.txt` file lists `STRINGS=Name` entries, and the tool
//! loads `Name_<lang>.strings` + `Name_<lang>.dlstrings` + `Name_<lang>.ilstrings`
//! for both source and target languages to build a translation corpus.

use std::path::Path;

use crate::strings::{CodepageTable, StringsFile};

/// Parse a `vocabulary.txt` file and return the list of STRINGS base names.
///
/// Format: lines starting with `STRINGS=` (case-sensitive) after stripping
/// comments (lines starting with `#`) and whitespace.
pub fn parse_vocabulary_file(path: &Path) -> Result<Vec<String>, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_vocabulary_content(&content))
}

/// Parse vocabulary content from a string.
fn parse_vocabulary_content(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                return None;
            }
            trimmed
                .strip_prefix("STRINGS=")
                .map(|s| s.trim().to_string())
        })
        .collect()
}

/// A vocabulary corpus: source→translation pairs extracted from game Strings files.
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    /// Source→translation pairs, keyed by (base_name, str_id) for dedup.
    pairs: Vec<(String, String)>,
}

impl Vocabulary {
    /// Build a vocabulary by loading source and target Strings files.
    ///
    /// - `names`: list of base names from vocabulary.txt (e.g. "Skyrim", "Update")
    /// - `strings_dir`: directory containing the Strings files
    /// - `source_lang`: source language (e.g. "english")
    /// - `target_lang`: target language (e.g. "chinese")
    /// - `codepage`: optional codepage table for decoding
    pub fn load(
        names: &[String],
        strings_dir: &Path,
        source_lang: &str,
        target_lang: &str,
        codepage: Option<&CodepageTable>,
    ) -> Self {
        let mut vocab = Self::default();
        for name in names {
            vocab.add_base_name(name, strings_dir, source_lang, target_lang, codepage);
        }
        vocab
    }

    /// Add one base name's worth of source→target pairs.
    fn add_base_name(
        &mut self,
        base_name: &str,
        strings_dir: &Path,
        source_lang: &str,
        target_lang: &str,
        codepage: Option<&CodepageTable>,
    ) {
        for ext in &["strings", "dlstrings", "ilstrings"] {
            let source_path = strings_dir.join(format!("{}_{}.{}", base_name, source_lang, ext));
            let target_path = strings_dir.join(format!("{}_{}.{}", base_name, target_lang, ext));

            if !source_path.exists() || !target_path.exists() {
                continue;
            }

            let source_file = match Self::load_strings_file(&source_path, codepage) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let target_file = match Self::load_strings_file(&target_path, codepage) {
                Ok(f) => f,
                Err(_) => continue,
            };

            // Match by str_id: source text → target text
            for (&id, source_text) in &source_file.strings {
                if source_text.is_empty() {
                    continue;
                }
                if let Some(target_text) = target_file.strings.get(&id) {
                    if !target_text.is_empty() {
                        self.pairs.push((source_text.clone(), target_text.clone()));
                    }
                }
            }
        }
    }

    fn load_strings_file(
        path: &Path,
        codepage: Option<&CodepageTable>,
    ) -> Result<StringsFile, anyhow::Error> {
        let result = match codepage {
            Some(table) => StringsFile::load_with_codepage_table(path, table),
            None => StringsFile::load(path),
        };
        result.map_err(|e| anyhow::anyhow!("Failed to load {}: {}", path.display(), e))
    }

    /// Return the source→translation pairs as a slice.
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    /// Return the number of pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether the vocabulary is empty.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vocabulary_content() {
        let content = "\
#list of strings pairs
#comment line
STRINGS=Update
STRINGS=Dawnguard
STRINGS=Skyrim

STRINGS=hearthfires
";
        let names = parse_vocabulary_content(content);
        assert_eq!(names, vec!["Update", "Dawnguard", "Skyrim", "hearthfires"]);
    }

    #[test]
    fn test_parse_vocabulary_ignores_comments() {
        let content = "\
# This is a comment
STRINGS=Skyrim
# Another comment
STRINGS=Update
";
        let names = parse_vocabulary_content(content);
        assert_eq!(names, vec!["Skyrim", "Update"]);
    }

    #[test]
    fn test_parse_vocabulary_empty() {
        let content = "\
# only comments
# nothing useful
";
        let names = parse_vocabulary_content(content);
        assert!(names.is_empty());
    }

    #[test]
    fn test_vocabulary_default_empty() {
        let vocab = Vocabulary::default();
        assert!(vocab.is_empty());
        assert_eq!(vocab.len(), 0);
    }
}
