//! PEX type definitions

/// PEX file header (starts after magic 0xFA57C0DE)
#[derive(Clone, Debug)]
pub struct PexHeader {
    pub major_version: u8,
    pub minor_version: u8,
    pub game_id: u16,
    /// Compilation time (mod_time from debug info section)
    pub compile_time: u64,
}

/// A string reference in the PEX string table
#[derive(Clone, Debug)]
pub struct PexStringEntry {
    pub index: u16,
    pub text: String,
}

/// Extracted translatable string
#[derive(Clone, Debug, PartialEq)]
pub struct PexTranslatableString {
    /// Name of the containing script object
    pub object_name: String,
    /// State name (empty for default state)
    pub state_name: String,
    /// Function name (empty for object-level docs)
    pub function_name: String,
    /// Type of string: "DebugString", "PropertyName", or "StringLiteral"
    pub string_type: String,
    /// The original text to translate
    pub source_text: String,
    /// The translated text (empty if not yet translated)
    pub translation: String,
}

/// Parsed PEX script information
#[derive(Clone, Debug)]
pub struct PexScript {
    pub header: PexHeader,
    /// The full string table (index -> text)
    pub string_table: Vec<PexStringEntry>,
    /// All extracted translatable strings
    pub translatable: Vec<PexTranslatableString>,
    /// Raw debug info section bytes (for preservation during recompile)
    pub debug_info_raw: Vec<u8>,
    /// Raw user flags section bytes (for preservation during recompile)
    pub user_flags_raw: Vec<u8>,
    /// Raw object body bytes per object (for preservation during recompile)
    pub object_bodies_raw: Vec<Vec<u8>>,
}
