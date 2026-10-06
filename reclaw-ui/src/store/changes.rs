//! The small edits the reducer makes to lists inside the state, as functions on plain data.
use std::collections::BTreeMap;

use reclaw_games::settings::{SettingKey, SettingValue, SettingsLayer};

use crate::model::{GameEntry, ModEntry, ModProvider, ModStatus};

const FAVORITE: &str = "favorite";

/// Make every game's favorite tag agree with the saved set. Returns whether anything changed.
pub fn sync_favorite_tags(games: &mut [GameEntry], favorites: &std::collections::BTreeSet<u32>) -> bool {
    let mut changed = false;
    for game in games {
        let marked = game.tags.iter().position(|t| t == FAVORITE);
        match (marked, favorites.contains(&game.id)) {
            (None, true) => {
                game.tags.push(FAVORITE.into());
                changed = true;
            }
            (Some(at), false) => {
                game.tags.remove(at);
                changed = true;
            }
            _ => {}
        }
    }
    changed
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

/// Show a mod as installing the moment Install or Update is pressed; the host sends the list again when the job ends.
/// Returns whether anything changed.
pub fn mark_mod_installing(mods: &mut [ModEntry], game: u32, provider: ModProvider, id: &str) -> bool {
    let waiting = |s: ModStatus| matches!(s, ModStatus::Available | ModStatus::UpdateReady);
    match mods.iter_mut().find(|m| m.game_id == game && m.provider == provider && m.id == id && waiting(m.status)) {
        Some(entry) => {
            entry.status = ModStatus::Installing;
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods_game(mods: &[ModEntry], id: &str) -> u32 {
        mods.iter().find(|m| m.id == id).map_or(0, |m| m.game_id)
    }
    use crate::fixtures::{sample_games, sample_mods};

    #[test]
    fn tags_follow_the_saved_set_both_ways() {
        let mut games = sample_games();
        let mut favorites = std::collections::BTreeSet::from([1]);
        assert!(sync_favorite_tags(&mut games, &favorites));
        assert!(games[0].is_favorite() && !games[1].is_favorite());
        assert!(!sync_favorite_tags(&mut games, &favorites), "nothing left to change");
        favorites.clear();
        assert!(sync_favorite_tags(&mut games, &favorites));
        assert!(!games[0].is_favorite());
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
        let game = mods_game(&mods, &id);
        assert!(!mark_mod_installing(&mut mods, game, provider, &id));
        assert_eq!(mods[installed].status, ModStatus::Installed);

        let available = mods.iter().position(|m| m.status == ModStatus::Available).expect("a sample available mod");
        let (provider, id) = (mods[available].provider, mods[available].id.clone());
        let game = mods_game(&mods, &id);
        assert!(mark_mod_installing(&mut mods, game, provider, &id));
        assert_eq!(mods[available].status, ModStatus::Installing);
    }
}
