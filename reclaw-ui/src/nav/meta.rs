//! What the app knows about a route without drawing it: which tab it belongs to, how it relates to
//! other pages, how it prefers to animate in. Pure, so the rules are unit-tested.
use super::{Route, Section, transition::TransitionStyle};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageKind {
    /// A top-level destination: switching between roots replaces rather than stacks.
    Root,
    /// A page about one thing (a game, a mod).
    Detail,
    /// A page that collects input and ends; kept out of the recent pages.
    Form,
    /// Settings: a list and the sections under it.
    Settings,
}

impl Route {
    pub fn kind(&self) -> PageKind {
        match self {
            Self::Library {} | Self::Catalog {} | Self::Mods {} | Self::Downloads {} => PageKind::Root,
            Self::Game { .. } | Self::ModDetail { .. } => PageKind::Detail,
            Self::Install { .. } => PageKind::Form,
            Self::GameSettings { .. } | Self::GameSettingsSection { .. } | Self::Settings {} | Self::SettingsSection { .. } => {
                PageKind::Settings
            }
        }
    }

    pub fn is_root(&self) -> bool {
        self.kind() == PageKind::Root
    }

    /// The tab this page belongs to, for pages that belong to one. A game page does not: it is
    /// reached from the Library or the Catalog, so the shell keeps the tab the user came from.
    pub fn section(&self) -> Option<Section> {
        match self {
            Self::Library {} => Some(Section::Library),
            Self::Catalog {} => Some(Section::Catalog),
            Self::Downloads {} => Some(Section::Downloads),
            Self::Mods {} | Self::ModDetail { .. } => Some(Section::Mods),
            _ => None,
        }
    }

    /// The route for a tab.
    pub fn of_section(section: Section) -> Self {
        match section {
            Section::Library => Self::Library {},
            Section::Catalog => Self::Catalog {},
            Section::Downloads => Self::Downloads {},
            Section::Mods => Self::Mods {},
        }
    }

    /// The page one level up, used when there is no history to go back through (a page opened by
    /// a link or at startup). Roots have none.
    pub fn up(&self) -> Option<Route> {
        match self {
            Self::Install { id } | Self::GameSettings { id } => Some(Self::Game { id: *id }),
            Self::GameSettingsSection { id, .. } => Some(Self::GameSettings { id: *id }),
            Self::ModDetail { .. } => Some(Self::Mods {}),
            Self::Settings {} => Some(Self::Library {}),
            Self::SettingsSection { .. } => Some(Self::Settings {}),
            Self::Game { .. } => Some(Self::Library {}),
            Self::Library {} | Self::Catalog {} | Self::Mods {} | Self::Downloads {} => None,
        }
    }

    /// A name that needs no data, for places that cannot look the game up.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Library {} => "Library",
            Self::Catalog {} => "Catalog",
            Self::Game { .. } => "Game",
            Self::Install { .. } => "Install",
            Self::GameSettings { .. } | Self::GameSettingsSection { .. } => "Game settings",
            Self::Mods {} => "Mods",
            Self::ModDetail { .. } => "Mod",
            Self::Downloads {} => "Downloads",
            Self::Settings {} | Self::SettingsSection { .. } => "Settings",
        }
    }

    /// The game this page is about, if it is about one.
    pub fn game_id(&self) -> Option<u32> {
        match self {
            Self::Game { id } | Self::Install { id } | Self::GameSettings { id } | Self::GameSettingsSection { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// How this page prefers to appear. `None` means "as the interface's default". A game page
    /// asks for `Hero` so its banner and artwork are the thing that arrives; going back from it
    /// plays the same style in reverse.
    pub fn enter_style(&self) -> Option<TransitionStyle> {
        match self {
            Self::Game { .. } => Some(TransitionStyle::Hero),
            Self::ModDetail { .. } => Some(TransitionStyle::Rise),
            _ => None,
        }
    }

    /// Whether the page is worth coming back to from the recent-pages list.
    pub fn is_recent_worthy(&self) -> bool {
        self.kind() != PageKind::Form
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn all_samples() -> Vec<Route> {
        vec![
            Route::Library {},
            Route::Catalog {},
            Route::Game { id: 4 },
            Route::Install { id: 4 },
            Route::GameSettings { id: 4 },
            Route::GameSettingsSection { id: 4, section: "display".into() },
            Route::Mods {},
            Route::ModDetail { provider: "thunderstore".into(), mod_id: "abc".into() },
            Route::Downloads {},
            Route::Settings {},
            Route::SettingsSection { section: "interface".into() },
        ]
    }

    #[test]
    fn every_route_round_trips_through_its_path() {
        for route in all_samples() {
            let path = route.to_string();
            assert_eq!(Route::from_str(&path).ok().as_ref(), Some(&route), "{path}");
        }
    }

    #[test]
    fn paths_are_the_documented_ones() {
        assert_eq!(Route::Library {}.to_string(), "/");
        assert_eq!(Route::Game { id: 4 }.to_string(), "/game/4");
        assert_eq!(Route::GameSettingsSection { id: 4, section: "display".into() }.to_string(), "/game/4/settings/display");
        assert_eq!(Route::ModDetail { provider: "thunderstore".into(), mod_id: "abc".into() }.to_string(), "/mods/thunderstore/abc");
    }

    #[test]
    fn up_always_reaches_a_root_without_looping() {
        for route in all_samples() {
            let mut at = route.clone();
            for _ in 0..8 {
                match at.up() {
                    Some(parent) => at = parent,
                    None => break,
                }
            }
            assert!(at.is_root(), "{route:?} ends at {at:?}");
        }
    }

    #[test]
    fn only_roots_are_roots_and_forms_are_not_recent() {
        for route in all_samples() {
            assert_eq!(route.is_root(), route.up().is_none(), "{route:?}");
        }
        assert!(!Route::Install { id: 1 }.is_recent_worthy());
        assert!(Route::Game { id: 1 }.is_recent_worthy());
    }

    #[test]
    fn sections_and_routes_map_both_ways() {
        for section in Section::ALL {
            assert_eq!(Route::of_section(section).section(), Some(section));
        }
        assert_eq!(Route::Game { id: 1 }.section(), None);
        assert_eq!(Route::ModDetail { provider: "p".into(), mod_id: "m".into() }.section(), Some(Section::Mods));
    }

    #[test]
    fn game_pages_know_their_game() {
        assert_eq!(Route::GameSettingsSection { id: 7, section: "x".into() }.game_id(), Some(7));
        assert_eq!(Route::Mods {}.game_id(), None);
    }
}
