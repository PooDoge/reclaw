/// Which UI to start in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiMode {
    Desktop,
    Deck,
}

/// Who handles the Guide button. Under Steam it belongs to Steam (its overlay, its Steam Input),
/// and Reclaw must not also react to it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GuideOwner {
    Reclaw,
    Steam,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Environment {
    pub mode: UiMode,
    pub guide_owner: GuideOwner,
    /// Why, for the settings screen and bug reports.
    pub reason: String,
}

/// Decide the starting mode from environment variables. `get` is injected so this is testable.
///
/// The Steam and gamescope variables are the commonly documented ones but were **not verified on
/// real SteamOS hardware** here; `RECLAW_MODE` always wins so a wrong guess can be overridden.
pub fn detect_environment(get: impl Fn(&str) -> Option<String>) -> Environment {
    let set = |k: &str| get(k).is_some_and(|v| !v.is_empty() && v != "0");
    let guide_owner = if get("SteamGameId").is_some() || get("SteamAppId").is_some() {
        GuideOwner::Steam
    } else {
        GuideOwner::Reclaw
    };

    let (mode, reason) = match get("RECLAW_MODE").as_deref() {
        Some("deck") => (UiMode::Deck, "RECLAW_MODE=deck".to_string()),
        Some("desktop") => (UiMode::Desktop, "RECLAW_MODE=desktop".to_string()),
        _ if set("SteamDeck") => (UiMode::Deck, "SteamDeck is set".to_string()),
        _ if set("SteamGamepadUI") => (
            UiMode::Deck,
            "SteamGamepadUI is set (Steam gaming mode)".to_string(),
        ),
        _ if get("XDG_CURRENT_DESKTOP").is_some_and(|v| v.to_lowercase().contains("gamescope")) => {
            (UiMode::Deck, "running inside gamescope".to_string())
        }
        _ if set("GAMESCOPE_WAYLAND_DISPLAY") => {
            (UiMode::Deck, "GAMESCOPE_WAYLAND_DISPLAY is set".to_string())
        }
        _ => (UiMode::Desktop, "no console session detected".to_string()),
    };
    Environment {
        mode,
        guide_owner,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> Environment {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        detect_environment(|k| map.get(k).cloned())
    }

    #[test]
    fn plain_desktop() {
        let e = env(&[("XDG_CURRENT_DESKTOP", "KDE")]);
        assert_eq!(
            (e.mode, e.guide_owner),
            (UiMode::Desktop, GuideOwner::Reclaw)
        );
    }

    #[test]
    fn explicit_override_beats_detection() {
        assert_eq!(
            env(&[("SteamDeck", "1"), ("RECLAW_MODE", "desktop")]).mode,
            UiMode::Desktop
        );
        assert_eq!(env(&[("RECLAW_MODE", "deck")]).mode, UiMode::Deck);
    }

    #[test]
    fn steam_gaming_mode_is_deck_and_steam_owns_guide() {
        let e = env(&[("SteamGamepadUI", "1"), ("SteamGameId", "123")]);
        assert_eq!((e.mode, e.guide_owner), (UiMode::Deck, GuideOwner::Steam));
    }

    #[test]
    fn gamescope_session() {
        assert_eq!(
            env(&[("XDG_CURRENT_DESKTOP", "gamescope")]).mode,
            UiMode::Deck
        );
        assert_eq!(env(&[("SteamDeck", "0")]).mode, UiMode::Desktop);
    }
}
