/// 游戏类型枚举
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameId {
    Skyrim,
    SkyrimSE,
    Fallout4,
    FalloutNV,
    Fallout76,
    Starfield,
}

impl GameId {
    /// Canonical identifier shared by `Data/<name>/` and frontend `currentGame`.
    pub fn as_str(&self) -> &'static str {
        match self {
            GameId::Skyrim => "Skyrim",
            GameId::SkyrimSE => "SkyrimSE",
            GameId::Fallout4 => "Fallout4",
            GameId::FalloutNV => "FalloutNV",
            GameId::Fallout76 => "Fallout76",
            GameId::Starfield => "Starfield",
        }
    }

    /// Parses a user/frontend game alias case-insensitively.
    /// Unknown aliases return `None`; callers should resolve fallbacks explicitly.
    pub fn from_alias(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "skyrim" => Some(GameId::Skyrim),
            "skyrimse" | "skyrim se" => Some(GameId::SkyrimSE),
            "fallout4" | "fo4" => Some(GameId::Fallout4),
            "falloutnv" | "fonv" | "fallout nv" | "new vegas" => Some(GameId::FalloutNV),
            "fallout76" | "fo76" => Some(GameId::Fallout76),
            "starfield" | "sf" => Some(GameId::Starfield),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [GameId; 6] = [
        GameId::Skyrim,
        GameId::SkyrimSE,
        GameId::Fallout4,
        GameId::FalloutNV,
        GameId::Fallout76,
        GameId::Starfield,
    ];

    #[test]
    fn as_str_round_trips_through_from_alias() {
        for g in ALL {
            assert_eq!(GameId::from_alias(g.as_str()), Some(g));
        }
    }

    #[test]
    fn from_alias_is_case_insensitive_and_accepts_known_shorthands() {
        assert_eq!(GameId::from_alias("SKYRIMSE"), Some(GameId::SkyrimSE));
        assert_eq!(GameId::from_alias("skyrim se"), Some(GameId::SkyrimSE));
        assert_eq!(GameId::from_alias("fo4"), Some(GameId::Fallout4));
        assert_eq!(GameId::from_alias("FO76"), Some(GameId::Fallout76));
        assert_eq!(GameId::from_alias("sf"), Some(GameId::Starfield));
        assert_eq!(GameId::from_alias("new vegas"), Some(GameId::FalloutNV));
    }

    #[test]
    fn from_alias_rejects_unknown_or_empty() {
        assert_eq!(GameId::from_alias("oblivion"), None);
        assert_eq!(GameId::from_alias("morrowind"), None);
        assert_eq!(GameId::from_alias(""), None);
    }
}
