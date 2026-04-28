//! PEX (Papyrus Executable) script binary format parser
//!
//! Bethesda's Papyrus scripting language compiles source scripts (.psc) into
//! compiled binary .pex files. This module parses PEX files and extracts
//! translatable strings: DebugString (documentation), PropertyName, StringLiteral.
//!
//! Based on Delphi TESVT_scriptPex.pas and community format documentation.
//!
//! Scope: string extraction + v1.5 write-back support (string table reconstruction).

pub mod compile;
pub mod parser;
pub mod types;
