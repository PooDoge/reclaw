//! What a page is called in a list of places: "Starfall 64", not "Game". Route metadata cannot say
//! (`Route::label` needs no data), so this looks the title up in the library and the mod list.
use super::Route;
use crate::model::{GameEntry, ModEntry};

/// A name for `route` a person would recognize. A game or mod that is no longer known falls back to
/// the kind of page, so a stale entry in the recents list still reads sensibly.
pub fn title(route: &Route, games: &[GameEntry], mods: &[ModEntry]) -> String {
    let game = |id: u32| games.iter().find(|g| g.id == id).map(|g| g.title.to_string());
    match route {
        Route::Game { id } => game(*id).unwrap_or_else(|| route.label().to_string()),
        Route::Install { id } => game(*id).map_or_else(|| route.label().to_string(), |t| format!("Install {t}")),
        Route::GameSettings { id } | Route::GameSettingsSection { id, .. } => {
            game(*id).map_or_else(|| route.label().to_string(), |t| format!("{t} settings"))
        }
        Route::ModDetail { provider, mod_id } => {
            mods.iter().find(|m| m.matches(provider, mod_id)).map_or_else(|| route.label().to_string(), |m| m.title.clone())
        }
        other => other.label().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::{sample_games, sample_mods};

    fn name(route: Route) -> String {
        title(&route, &sample_games(), &sample_mods())
    }

    #[test]
    fn pages_about_a_game_carry_its_title() {
        assert_eq!(name(Route::Game { id: 1 }), "Starfall 64");
        assert_eq!(name(Route::Install { id: 4 }), "Install Dino Rush");
        assert_eq!(name(Route::GameSettings { id: 2 }), "Skyward Quest settings");
        assert_eq!(name(Route::GameSettingsSection { id: 2, section: "display".into() }), "Skyward Quest settings");
    }

    #[test]
    fn a_mod_page_carries_the_mods_title() {
        let first = sample_mods().remove(0);
        assert_eq!(name(Route::ModDetail { provider: first.provider.slug().to_string(), mod_id: first.id.clone() }), first.title);
    }

    #[test]
    fn the_tabs_are_called_what_they_always_are() {
        assert_eq!(name(Route::Library {}), "Library");
        assert_eq!(name(Route::Downloads {}), "Downloads");
        assert_eq!(name(Route::SettingsSection { section: "motion".into() }), "Settings");
    }

    #[test]
    fn something_no_longer_known_falls_back_to_the_kind_of_page() {
        assert_eq!(name(Route::Game { id: 999 }), "Game");
        assert_eq!(name(Route::Install { id: 999 }), "Install");
        assert_eq!(name(Route::ModDetail { provider: "thunderstore".into(), mod_id: "gone".into() }), "Mod");
    }
}
