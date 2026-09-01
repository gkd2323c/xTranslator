//! Detects the target game from the ESP/ESM itself.
//!
//! This mirrors Delphi xTranslator 1.6.0 `TESVT_Const.pas::getGameByFormVersion`.
//! The original does not guess from the file path: it reads the TES4 record header
//! Form Version (`RecordHeaderData::f_version`), maps its range to a game, and compares
//! that result with the user's selected workspace. A mismatch is reported rather than
//! silently switching workspaces.
//!
//! Detection failure is represented as `None`; callers must treat it as unknown.

use super::header::{GenericHeader, RecordHeaderData};
use crate::types::game_id::GameId;
use std::io::Read;
use std::path::Path;

/// Maps TES4 Form Version to a game using Delphi `getGameByFormVersion` ranges.
/// Values outside known ranges return `None` (Delphi returns `-1`).
pub fn game_from_form_version(f_version: u16) -> Option<GameId> {
    match f_version {
        131 => Some(GameId::Fallout4),
        40..=43 => Some(GameId::Skyrim),
        2..=15 => Some(GameId::FalloutNV),
        44 => Some(GameId::SkyrimSE),
        182..=201 => Some(GameId::Fallout76),
        552..=576 => Some(GameId::Starfield),
        _ => None,
    }
}

/// Reads only the 8-byte TES4 generic header and 16-byte record header data.
/// This is game-definition independent, so detection can happen before choosing `Data/<Game>`.
/// Returns `None` when the file does not begin with TES4 or the header cannot be read.
pub fn peek_form_version<R: Read>(reader: &mut R) -> Option<u16> {
    let generic = GenericHeader::read_from(reader).ok()?;
    if !generic.is_tes4() {
        return None;
    }
    let record_header = RecordHeaderData::read_from(reader).ok()?;
    Some(record_header.f_version)
}

/// Detects the game directly from a plugin path.
pub fn detect_game_from_esp(path: &Path) -> Option<GameId> {
    let mut file = std::fs::File::open(path).ok()?;
    let f_version = peek_form_version(&mut file)?;
    game_from_form_version(f_version)
}

/// Describes how the resolved `game_id` was chosen.
/// `Fallback` is intentionally untrusted and requires explicit user selection downstream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameSource {
    Requested,
    Detected,
    Fallback,
}

impl GameSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameSource::Requested => "requested",
            GameSource::Detected => "detected",
            GameSource::Fallback => "fallback",
        }
    }
}

/// Resolves a game using: explicit request > file detection > compatibility fallback.
/// The fallback source remains explicitly marked instead of masquerading as a trusted result.
pub fn resolve_game_id(
    requested: Option<&str>,
    detected: Option<GameId>,
    fallback: GameId,
) -> (GameId, GameSource) {
    if let Some(g) = requested.and_then(GameId::from_alias) {
        return (g, GameSource::Requested);
    }
    if let Some(g) = detected {
        return (g, GameSource::Detected);
    }
    (fallback, GameSource::Fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn maps_known_form_version_ranges() {
        assert_eq!(game_from_form_version(40), Some(GameId::Skyrim));
        assert_eq!(game_from_form_version(43), Some(GameId::Skyrim));
        assert_eq!(game_from_form_version(44), Some(GameId::SkyrimSE));
        assert_eq!(game_from_form_version(131), Some(GameId::Fallout4));
        assert_eq!(game_from_form_version(2), Some(GameId::FalloutNV));
        assert_eq!(game_from_form_version(15), Some(GameId::FalloutNV));
        assert_eq!(game_from_form_version(182), Some(GameId::Fallout76));
        assert_eq!(game_from_form_version(201), Some(GameId::Fallout76));
        assert_eq!(game_from_form_version(552), Some(GameId::Starfield));
        assert_eq!(game_from_form_version(576), Some(GameId::Starfield));
    }

    #[test]
    fn rejects_values_outside_any_known_range() {
        // Exercise values immediately outside ranges and gaps between ranges.
        for v in [0u16, 1, 16, 39, 45, 130, 132, 181, 202, 551, 577, 65535] {
            assert_eq!(game_from_form_version(v), None, "v={v} should be unmapped");
        }
    }

    fn tes4_bytes(f_version: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"TES4");
        buf.extend_from_slice(&0u32.to_le_bytes()); // dsize (field body is irrelevant here)
        buf.extend_from_slice(&0u32.to_le_bytes()); // flags
        buf.extend_from_slice(&0u32.to_le_bytes()); // form_id
        buf.extend_from_slice(&0u32.to_le_bytes()); // version (VCS metadata)
        buf.extend_from_slice(&f_version.to_le_bytes()); // f_version
        buf.extend_from_slice(&0u16.to_le_bytes()); // v_info
        buf
    }

    #[test]
    fn peek_form_version_reads_tes4_header_only() {
        let bytes = tes4_bytes(44);
        let mut cursor = Cursor::new(bytes);
        assert_eq!(peek_form_version(&mut cursor), Some(44));
        // Consume exactly 24 bytes: 8 generic header + 16 record header data.
        assert_eq!(cursor.position(), 24);
    }

    #[test]
    fn peek_form_version_rejects_non_tes4_records() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"GRUP");
        buf.extend_from_slice(&0u32.to_le_bytes());
        let mut cursor = Cursor::new(buf);
        assert_eq!(peek_form_version(&mut cursor), None);
    }

    #[test]
    fn resolve_game_id_prefers_requested_over_detected() {
        let (g, src) = resolve_game_id(Some("Fallout4"), Some(GameId::SkyrimSE), GameId::SkyrimSE);
        assert_eq!(g, GameId::Fallout4);
        assert_eq!(src, GameSource::Requested);
    }

    #[test]
    fn resolve_game_id_falls_back_to_detected_when_not_requested() {
        let (g, src) = resolve_game_id(None, Some(GameId::Starfield), GameId::SkyrimSE);
        assert_eq!(g, GameId::Starfield);
        assert_eq!(src, GameSource::Detected);
    }

    #[test]
    fn resolve_game_id_marks_fallback_explicitly_when_nothing_known() {
        let (g, src) = resolve_game_id(None, None, GameId::SkyrimSE);
        assert_eq!(g, GameId::SkyrimSE);
        assert_eq!(src, GameSource::Fallback);
    }

    #[test]
    fn resolve_game_id_ignores_unrecognized_requested_alias() {
        // An unknown explicit alias falls back to detection rather than a guessed default.
        let (g, src) = resolve_game_id(Some("oblivion"), Some(GameId::Fallout4), GameId::SkyrimSE);
        assert_eq!(g, GameId::Fallout4);
        assert_eq!(src, GameSource::Detected);
    }
}
