//! Which tab a search is for. Each tab keeps its own search, so searching the catalog does not empty the library.
use crate::nav::{Route, Section};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SearchScope {
    Library,
    Catalog,
    Mods,
    Settings,
}

impl SearchScope {
    pub const ALL: [SearchScope; 4] = [Self::Library, Self::Catalog, Self::Mods, Self::Settings];

    /// The search a tab has. Downloads has none: it is a short list of what is running.
    pub fn of_section(section: Section) -> Option<Self> {
        match section {
            Section::Library => Some(Self::Library),
            Section::Catalog => Some(Self::Catalog),
            Section::Mods => Some(Self::Mods),
            Section::Downloads => None,
        }
    }

    /// The scope whose own page `route` is: where a narrow window shows its floating search button. A game's page or a mod's
    /// page has buttons of its own at the top right, so it gets none there (the wide window's top bar has it everywhere).
    pub fn root_of(route: &Route) -> Option<Self> {
        match route {
            Route::Library {} => Some(Self::Library),
            Route::Catalog {} => Some(Self::Catalog),
            Route::Mods {} => Some(Self::Mods),
            Route::Settings {} | Route::SettingsSection { .. } => Some(Self::Settings),
            _ => None,
        }
    }

    /// The page a search's results are shown on: searching from a game's page goes back to its tab.
    pub fn route(self) -> Route {
        match self {
            Self::Library => Route::Library {},
            Self::Catalog => Route::Catalog {},
            Self::Mods => Route::Mods {},
            Self::Settings => Route::Settings {},
        }
    }

    /// The words in the empty box. `within` narrows it ("Search mods for Starfall 64").
    pub fn placeholder(self, within: Option<&str>) -> String {
        match (self, within) {
            (Self::Library, _) => "Search your library".to_string(),
            (Self::Catalog, _) => "Search the catalog".to_string(),
            (Self::Mods, Some(game)) => format!("Search mods for {game}"),
            (Self::Mods, None) => "Search mods".to_string(),
            (Self::Settings, _) => "Search settings".to_string(),
        }
    }

    /// "in your library": how a results line says where it looked.
    pub fn place(self) -> &'static str {
        match self {
            Self::Library => "in your library",
            Self::Catalog => "in the catalog",
            Self::Mods => "in mods",
            Self::Settings => "in settings",
        }
    }

    pub(crate) fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tab_but_downloads_has_a_search() {
        let scopes: Vec<_> = Section::ALL.iter().map(|s| SearchScope::of_section(*s)).collect();
        assert_eq!(scopes, vec![Some(SearchScope::Library), Some(SearchScope::Catalog), None, Some(SearchScope::Mods)]);
    }

    #[test]
    fn only_a_tabs_own_page_gets_the_floating_button() {
        assert_eq!(SearchScope::root_of(&Route::Mods {}), Some(SearchScope::Mods));
        assert_eq!(SearchScope::root_of(&Route::SettingsSection { section: "network".into() }), Some(SearchScope::Settings));
        assert_eq!(SearchScope::root_of(&Route::Game { id: 1 }), None);
        assert_eq!(SearchScope::root_of(&Route::Downloads {}), None);
    }

    #[test]
    fn results_show_on_the_tabs_page_and_the_box_says_what_it_searches() {
        assert_eq!(SearchScope::Catalog.route(), Route::Catalog {});
        assert_eq!(SearchScope::Mods.placeholder(Some("Starfall 64")), "Search mods for Starfall 64");
        assert_eq!(SearchScope::Mods.placeholder(None), "Search mods");
        assert!(SearchScope::ALL.iter().enumerate().all(|(i, s)| s.index() == i));
    }
}
