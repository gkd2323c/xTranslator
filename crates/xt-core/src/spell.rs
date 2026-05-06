//! Spell check module — Hunspell-based spell checking for translated text.
//!
//! Architecture matches Delphi TESVT_SpellCheck.pas:
//! - Tag-aware word splitting (skip `<...>` content)
//! - Parse options: ignore first-uppercase, multi-uppercase, alias tags
//! - Hash cache for correct/faulty words (binary search, FNV-1a hashes)
//! - Fault-ratio lockout to avoid false-positive floods
//! - Persistent ignore list
//! - Suggestions via Hunspell Suggest()

use std::collections::HashMap;
use std::path::Path;

/// Parse options for word extraction
#[derive(Debug, Clone, Copy, Default)]
pub struct SpellParseOptions {
    pub ignore_first_upper: bool,
    pub ignore_multi_upper: bool,
    pub ignore_alias_tags: bool,
}

#[derive(Debug, Clone)]
pub struct SpellWord {
    pub word: String,
    pub hash: u64,
    pub start_byte: usize,
    pub end_byte: usize,
    pub is_fault: bool,
    pub in_tag: bool,
}

#[derive(Debug, Clone)]
pub struct SpellResult {
    pub fault_words: Vec<SpellWord>,
    pub total_words: usize,
    pub fault_ratio_locked: bool,
}

#[derive(Debug, Clone)]
pub struct SpellCheckConfig {
    pub available_dictionaries: Vec<String>,
    pub current_dictionary: Option<String>,
    pub active: bool,
    pub loaded: bool,
}

const MAX_UNDERLINES: usize = 100;
const MAX_UNDERLINE_RATIO: usize = 30; // percentage

/// A runtime-loaded Hunspell instance.
struct HunspellHandle {
    lib: libloading::Library,
    handle: *mut std::ffi::c_void,
}

unsafe impl Send for HunspellHandle {}
unsafe impl Sync for HunspellHandle {}

impl HunspellHandle {
    fn load(dll_path: &str, aff_path: &str, dic_path: &str) -> Result<Self, String> {
        unsafe {
            let lib = libloading::Library::new(dll_path)
                .map_err(|e| format!("Failed to load Hunspell DLL ({}): {}", dll_path, e))?;

            let create: libloading::Symbol<
                unsafe extern "C" fn(
                    affpath: *const std::os::raw::c_char,
                    dpath: *const std::os::raw::c_char,
                ) -> *mut std::ffi::c_void,
            > = lib
                .get(b"Hunspell_create")
                .map_err(|e| format!("Hunspell_create not found: {}", e))?;

            let aff_c = std::ffi::CString::new(aff_path)
                .map_err(|e| format!("Invalid aff path: {}", e))?;
            let dic_c = std::ffi::CString::new(dic_path)
                .map_err(|e| format!("Invalid dic path: {}", e))?;

            let handle = create(aff_c.as_ptr(), dic_c.as_ptr());
            if handle.is_null() {
                return Err("Hunspell_create returned null".to_string());
            }

            Ok(Self { lib, handle })
        }
    }

    fn spell(&self, word: &str) -> bool {
        unsafe {
            let spell_fn: libloading::Symbol<
                unsafe extern "C" fn(
                    handle: *mut std::ffi::c_void,
                    word: *const std::os::raw::c_char,
                ) -> std::os::raw::c_int,
            > = match self.lib.get(b"Hunspell_spell") {
                Ok(f) => f,
                Err(_) => return true, // assume correct if can't check
            };

            let word_c = match std::ffi::CString::new(word) {
                Ok(c) => c,
                Err(_) => return true,
            };

            spell_fn(self.handle, word_c.as_ptr()) != 0
        }
    }

    fn suggest(&self, word: &str) -> Vec<String> {
        unsafe {
            let suggest_fn: libloading::Symbol<
                unsafe extern "C" fn(
                    handle: *mut std::ffi::c_void,
                    slst: *mut *mut *mut std::os::raw::c_char,
                    word: *const std::os::raw::c_char,
                ) -> std::os::raw::c_int,
            > = match self.lib.get(b"Hunspell_suggest") {
                Ok(f) => f,
                Err(_) => return vec![],
            };

            let free_list_fn: libloading::Symbol<
                unsafe extern "C" fn(
                    handle: *mut std::ffi::c_void,
                    slst: *mut *mut *mut std::os::raw::c_char,
                    n: std::os::raw::c_int,
                ),
            > = match self.lib.get(b"Hunspell_free_list") {
                Ok(f) => f,
                Err(_) => return vec![],
            };

            let word_c = match std::ffi::CString::new(word) {
                Ok(c) => c,
                Err(_) => return vec![],
            };

            let mut slist: *mut *mut std::os::raw::c_char = std::ptr::null_mut();
            let n = suggest_fn(self.handle, &mut slist, word_c.as_ptr());

            if n <= 0 || slist.is_null() {
                return vec![];
            }

            let mut result = Vec::with_capacity(n as usize);
            for i in 0..n as isize {
                let s_ptr = *slist.offset(i);
                if !s_ptr.is_null() {
                    let suggestion = std::ffi::CStr::from_ptr(s_ptr)
                        .to_string_lossy()
                        .into_owned();
                    result.push(suggestion);
                }
            }

            free_list_fn(self.handle, &mut slist, n);
            result
        }
    }
}

impl Drop for HunspellHandle {
    fn drop(&mut self) {
        unsafe {
            let destroy: Result<
                libloading::Symbol<unsafe extern "C" fn(handle: *mut std::ffi::c_void)>,
                _,
            > = self.lib.get(b"Hunspell_destroy");
            if let Ok(destroy_fn) = destroy {
                destroy_fn(self.handle);
            }
        }
    }
}

/// Main spell checker
pub struct SpellChecker {
    hunspell: Option<HunspellHandle>,
    correct_cache: HashMap<u64, ()>,
    fault_cache: HashMap<u64, ()>,
    ignore_list: Vec<String>,
    pub config: SpellCheckConfig,
    pub parse_options: SpellParseOptions,
    pub fault_ratio_locked: bool,
}

impl SpellChecker {
    pub fn new() -> Self {
        Self {
            hunspell: None,
            correct_cache: HashMap::new(),
            fault_cache: HashMap::new(),
            ignore_list: Vec::new(),
            config: SpellCheckConfig {
                available_dictionaries: Vec::new(),
                current_dictionary: None,
                active: false,
                loaded: false,
            },
            parse_options: SpellParseOptions::default(),
            fault_ratio_locked: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.config.active && self.config.loaded
    }

    /// Load Hunspell DLL and dictionary.
    pub fn load(&mut self, dll_path: &str, dic_dir: &str, dict_name: &str) -> Result<(), String> {
        let dic_path = Path::new(dic_dir).join(format!("{}.dic", dict_name));
        let aff_path = Path::new(dic_dir).join(format!("{}.aff", dict_name));

        let dic_str = dic_path.to_string_lossy().to_string();
        let aff_str = aff_path.to_string_lossy().to_string();

        if !Path::new(&aff_str).exists() {
            return Err(format!("Affix file not found: {}", aff_str));
        }
        if !Path::new(&dic_str).exists() {
            return Err(format!("Dictionary file not found: {}", dic_str));
        }

        let handle = HunspellHandle::load(dll_path, &aff_str, &dic_str)?;
        self.hunspell = Some(handle);
        self.config.current_dictionary = Some(dict_name.to_string());
        self.config.loaded = true;

        // Rebuild caches from ignore list
        self.rebuild_cache();

        Ok(())
    }

    /// Unload the Hunspell backend.
    pub fn unload(&mut self) {
        self.hunspell = None;
        self.config.loaded = false;
        self.correct_cache.clear();
        self.fault_cache.clear();
        self.fault_ratio_locked = false;
    }

    /// Scan a directory for available dictionaries.
    pub fn scan_dictionaries(dic_dir: &str) -> Vec<String> {
        let dir = Path::new(dic_dir);
        if !dir.exists() {
            return vec![];
        }
        let mut dicts = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "dic") {
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy().to_string();
                        // Check for matching .aff file
                        let aff_path = dir.join(format!("{}.aff", name));
                        if aff_path.exists() {
                            dicts.push(name);
                        }
                    }
                }
            }
        }
        dicts.sort();
        dicts
    }

    /// Add a word to the persistent ignore list.
    pub fn add_ignore(&mut self, word: &str) {
        if !self.ignore_list.iter().any(|w| w.eq_ignore_ascii_case(word)) {
            self.ignore_list.push(word.to_string());
        }
        let hash = hash_word(word);
        self.correct_cache.insert(hash, ());
    }

    /// Load ignore list from file.
    pub fn load_ignore_list(&mut self, path: &str) {
        if let Ok(content) = std::fs::read_to_string(path) {
            self.ignore_list = content.lines().map(|s| s.to_string()).collect();
        }
    }

    /// Save ignore list to file.
    pub fn save_ignore_list(&self, path: &str) -> std::io::Result<()> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.ignore_list.join("\n"))
    }

    fn rebuild_cache(&mut self) {
        self.correct_cache.clear();
        for word in &self.ignore_list {
            self.correct_cache.insert(hash_word(word), ());
        }
    }

    /// Check spelling of a single word.
    fn check_word(&mut self, word: &str) -> bool {
        if word.len() <= 1 {
            return true;
        }
        let hash = hash_word(word);

        // Check caches first
        if self.correct_cache.contains_key(&hash) {
            return true;
        }
        if self.fault_cache.contains_key(&hash) {
            return false;
        }

        // Check Hunspell
        let is_correct = match &self.hunspell {
            Some(h) => h.spell(word),
            None => true,
        };

        if is_correct {
            self.correct_cache.insert(hash, ());
        } else {
            self.fault_cache.insert(hash, ());
        }

        is_correct
    }

    /// Analyze text and return spell check result.
    pub fn analyze(&mut self, text: &str) -> SpellResult {
        let words = split_text_into_words(text, &self.parse_options);

        let total_words = words.len();
        let mut fault_words = Vec::new();

        for mut w in words {
            if w.in_tag || w.word.len() <= 1 {
                continue;
            }

            if !self.check_word(&w.word) {
                w.is_fault = true;
                fault_words.push(w);
            }

            if fault_words.len() > MAX_UNDERLINES {
                break;
            }
        }

        // Fault ratio lockout
        if total_words > 0 && !self.fault_ratio_locked {
            let ratio = (fault_words.len() * 100) / total_words;
            self.fault_ratio_locked =
                fault_words.len() > MAX_UNDERLINES && ratio > MAX_UNDERLINE_RATIO;
        }

        SpellResult {
            fault_words,
            total_words,
            fault_ratio_locked: self.fault_ratio_locked,
        }
    }

    /// Get suggestions for a misspelled word.
    pub fn suggestions(&self, word: &str) -> Vec<String> {
        match &self.hunspell {
            Some(h) => h.suggest(word),
            None => vec![],
        }
    }

    /// Reset internal caches (but keep ignore list).
    pub fn reset_caches(&mut self) {
        self.correct_cache.clear();
        self.fault_cache.clear();
        self.fault_ratio_locked = false;
        self.rebuild_cache();
    }
}

// ── Word splitting ───────────────────────────────────────────────

const WORD_DELIMITERS: &[char] = &[
    ' ', '\t', '\r', '\n', '.', ',', '!', '?', ':', ';',
    '-', '\'', '"', '(', ')', '[', ']', '{', '}', '/', '\\',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    '+', '=', '*', '&', '%', '$', '#', '@', '~', '`', '|',
];

const END_LINE_CHARS: &[char] = &['.', '!', '?', '\r', '\n'];

fn is_delimiter(c: char) -> bool {
    WORD_DELIMITERS.contains(&c) || c == '<' || c == '>'
}

fn is_end_line(c: char) -> bool {
    END_LINE_CHARS.contains(&c)
}

fn is_upper(c: char) -> bool {
    c.is_uppercase()
}

fn split_text_into_words(text: &str, opts: &SpellParseOptions) -> Vec<SpellWord> {
    let mut words = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut pos = 0usize;
    let mut in_tag = false;
    let mut tag_depth = 0u32;
    let mut start_line = true;

    while pos < len {
        let ch = text[pos..].chars().next().unwrap_or(' ');
        let ch_len = ch.len_utf8();

        if ch == '<' && !in_tag {
            in_tag = true;
            tag_depth = 1;
            pos += ch_len;
            continue;
        }

        if ch == '<' && in_tag {
            tag_depth += 1;
            pos += ch_len;
            continue;
        }

        if ch == '>' && in_tag {
            tag_depth -= 1;
            if tag_depth == 0 {
                in_tag = false;
            }
            pos += ch_len;
            continue;
        }

        // Inside tag: skip all content
        if in_tag {
            pos += ch_len;
            continue;
        }

        // Skip delimiters outside tags
        if is_delimiter(ch) {
            if is_end_line(ch) {
                start_line = true;
            }
            pos += ch_len;
            continue;
        }

        // Collect word
        let word_start = pos;
        while pos < len {
            let next_ch = text[pos..].chars().next().unwrap_or(' ');
            let next_len = next_ch.len_utf8();
            if next_ch == '<' || next_ch == '>' || is_delimiter(next_ch) {
                break;
            }
            pos += next_len;
        }
        let word_end = pos;

        if word_end > word_start {
            let raw_word = &text[word_start..word_end];
            let cleaned = raw_word.trim_matches(&['-', '\'', '_']);

            if cleaned.len() >= 2 {
                let delete = should_delete_word(cleaned, start_line, opts);
                if !delete {
                    let hash = hash_word(cleaned);
                    words.push(SpellWord {
                        word: cleaned.to_string(),
                        hash,
                        start_byte: word_start,
                        end_byte: word_end,
                        is_fault: false,
                        in_tag: false,
                    });
                }
            }
        }

        start_line = false;
    }

    words
}

fn should_delete_word(word: &str, start_line: bool, opts: &SpellParseOptions) -> bool {
    // Words with numbers are skipped
    if word.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }

    // Words with multiple uppercase chars (acronyms)
    if opts.ignore_multi_upper {
        let upper_count = word.chars().filter(|c| is_upper(*c)).count();
        if upper_count >= 2 && word.len() > 2 {
            return true;
        }
    }

    // First uppercase at line start
    if opts.ignore_first_upper && start_line && word.len() > 1 {
        if let Some(first) = word.chars().next() {
            if is_upper(first) {
                // Only skip if rest is lowercase
                let rest_lower = word.chars().skip(1).all(|c| !is_upper(c));
                if rest_lower {
                    return true;
                }
            }
        }
    }

    false
}

fn hash_word(word: &str) -> u64 {
    // FNV-1a 64-bit
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in word.to_lowercase().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let h1 = hash_word("hello");
        let h2 = hash_word("Hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_split_simple() {
        let words = split_text_into_words("Hello World", &SpellParseOptions::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].word, "Hello");
        assert_eq!(words[1].word, "World");
    }

    #[test]
    fn test_split_skip_tags() {
        let opts = SpellParseOptions {
            ignore_alias_tags: true,
            ..Default::default()
        };
        let words = split_text_into_words("Press <Alias=Button> to continue", &opts);
        assert_eq!(words.len(), 3);
        assert!(!words.iter().any(|w| w.word.contains("Alias")));
    }

    #[test]
    fn test_split_skip_numbers() {
        // Digits are delimiters; "Test123" splits to "Test" + "123"
        let words = split_text_into_words("Test123 abc", &SpellParseOptions::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].word, "Test");
        assert_eq!(words[1].word, "abc");
    }

    #[test]
    fn test_ignore_first_upper() {
        let opts = SpellParseOptions {
            ignore_first_upper: true,
            ..Default::default()
        };
        let words = split_text_into_words("Hello world", &opts);
        // "Hello" is at line start and has uppercase first letter → skipped
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].word, "world");
    }

    #[test]
    fn test_ignore_multi_upper() {
        let opts = SpellParseOptions {
            ignore_multi_upper: true,
            ..Default::default()
        };
        let words = split_text_into_words("NPC Dialog", &opts);
        // "NPC" has multiple uppercase → skipped
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].word, "Dialog");
    }

    #[test]
    fn test_scan_dictionaries() {
        let dir = std::env::temp_dir();
        // Should return empty for non-existent dict dir
        let dicts = SpellChecker::scan_dictionaries(dir.to_str().unwrap());
        assert!(dicts.is_empty());
    }

    #[test]
    fn test_checker_new() {
        let checker = SpellChecker::new();
        assert!(!checker.is_active());
        assert!(!checker.config.loaded);
    }
}
