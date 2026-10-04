//! Edits to the shared state that belong to the UI, as functions on plain data so they are
//! unit-tested without a window.
use std::collections::BTreeMap;

use reclaw_games::settings::{SettingKey, SettingValue, SettingsLayer};

use crate::model::{GameEntry, ModEntry, ModProvider, ModStatus};

const FAVORITE: &str = "favorite";

/// Add or remove the favorite mark. Returns whether the game is now a favorite (false if unknown).
pub fn toggle_favorite(games: &mut [GameEntry], id: u32) -> bool {
    let Some(game) = games.iter_mut().find(|g| g.id == id) else { return false };
    if let Some(at) = game.tags.iter().position(|t| t == FAVORITE) {
        game.tags.remove(at);
        false
    } else {
        game.tags.push(FAVORITE.into());
        true
    }
}

/// Set (`Some`) or clear (`None`) one launch setting at the defaults level (`app: None`) or for one
/// game. A game's layer is dropped once it holds nothing, so "no overrides" stays a missing entry.
pub fn set_launch_setting(
    defaults: &mut SettingsLayer,
    overrides: &mut BTreeMap<u32, SettingsLayer>,
    app: Option<u32>,
    key: SettingKey,
    value: Option<SettingValue>,
) {
    let apply = |layer: &mut SettingsLayer| match value.clone() {
        Some(v) => layer.set(key, v),
        None => {
            layer.clear(key);
        }
    };
    match app {
        None => apply(defaults),
        Some(id) => {
            apply(overrides.entry(id).or_default());
            if overrides.get(&id).is_some_and(SettingsLayer::is_empty) {
                overrides.remove(&id);
            }
        }
    }
}

/// Show a mod as installing the moment the button is pressed; the host moves it to `Installed`.
pub fn mark_mod_installing(mods: &mut [ModEntry], provider: ModProvider, id: &str) {
    if let Some(entry) = mods.iter_mut().find(|m| m.provider == provider && m.id == id && m.status == ModStatus::Available) {
        entry.status = ModStatus::Installing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::{sample_games, sample_mods};

    #[test]
    fn favorite_toggles_and_unknown_games_are_ignored() {
        let mut games = sample_games();
        assert!(toggle_favorite(&mut games, 1));
        assert!(games[0].is_favorite());
        assert!(!toggle_favorite(&mut games, 1));
        assert!(!games[0].is_favorite());
        assert!(!toggle_favorite(&mut games, 999));
    }

    #[test]
    fn a_game_override_is_created_and_dropped_when_empty() {
        let (mut defaults, mut overrides) = (SettingsLayer::default(), BTreeMap::new());
        set_launch_setting(&mut defaults, &mut overrides, Some(3), SettingKey::Vsync, Some(SettingValue::Bool(false)));
        assert_eq!(overrides[&3].get(SettingKey::Vsync), Some(&SettingValue::Bool(false)));
        assert!(defaults.is_empty(), "an override does not touch the defaults");
        set_launch_setting(&mut defaults, &mut overrides, Some(3), SettingKey::Vsync, None);
        assert!(overrides.is_empty());
    }

    #[test]
    fn defaults_apply_without_an_app() {
        let (mut defaults, mut overrides) = (SettingsLayer::default(), BTreeMap::new());
        set_launch_setting(&mut defaults, &mut overrides, None, SettingKey::Msaa, Some(SettingValue::choice("4x")));
        assert_eq!(defaults.get(SettingKey::Msaa), Some(&SettingValue::choice("4x")));
        assert!(overrides.is_empty());
    }

    #[test]
    fn only_an_available_mod_becomes_installing() {
        let mut mods = sample_mods();
        let installed = mods.iter().position(|m| m.status == ModStatus::Installed).expect("a sample installed mod");
        let (provider, id) = (mods[installed].provider, mods[installed].id.clone());
        mark_mod_installing(&mut mods, provider, &id);
        assert_eq!(mods[installed].status, ModStatus::Installed);

        let available = mods.iter().position(|m| m.status == ModStatus::Available).expect("a sample available mod");
        let (provider, id) = (mods[available].provider, mods[available].id.clone());
        mark_mod_installing(&mut mods, provider, &id);
        assert_eq!(mods[available].status, ModStatus::Installing);
    }
}
