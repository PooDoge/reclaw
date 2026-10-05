//! What an empty list says. With real data, empty is a normal state (the first start, the catalog still loading, nothing added yet),
//! and each of its causes needs its own words: "No projects match" for a catalog that has not arrived would blame a search nobody made.
use crate::store::{CatalogPhase, CatalogStatus};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EmptyText {
    pub title: &'static str,
    pub text: &'static str,
}

const fn say(title: &'static str, text: &'static str) -> EmptyText {
    EmptyText { title, text }
}

/// The Catalog with nothing to show. `total` is how many projects the catalog has before any search or platform filter.
pub fn catalog(status: &CatalogStatus, total: usize) -> EmptyText {
    match (total, status.phase) {
        (0, CatalogPhase::Idle | CatalogPhase::Loading) => say("Loading the catalog", "The list of recompilation projects is on its way."),
        (0, CatalogPhase::Failed) => {
            say("The catalog could not be loaded", "Check the connection and press Refresh. The notification has the details.")
        }
        (0, CatalogPhase::Ready) => say("The catalog lists no projects", "The catalog index in use is empty."),
        _ => say("No projects match", "Try another platform or a different search."),
    }
}

/// The Library with nothing in it. `filtered` is true when the library has games and a filter or search hides them all.
pub fn library(filtered: bool) -> EmptyText {
    if filtered {
        say("No games match", "Try another filter or a different search.")
    } else {
        say("Your library is empty", "Add games from the Catalog and they will be listed here.")
    }
}

/// The Mods tab with nothing to show. `any_known` is whether the program knows of any mod at all.
pub fn mods(any_known: bool) -> EmptyText {
    if any_known {
        say("No mods match", "Try another site or a different search.")
    } else {
        say("No mods to show yet", "Mod listings from Thunderstore and GameBanana are not built yet.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(phase: CatalogPhase) -> CatalogStatus {
        CatalogStatus { phase, ..Default::default() }
    }

    #[test]
    fn a_catalog_that_has_not_arrived_does_not_blame_the_search() {
        for phase in [CatalogPhase::Idle, CatalogPhase::Loading] {
            assert_eq!(catalog(&status(phase), 0).title, "Loading the catalog");
        }
        assert_eq!(catalog(&status(CatalogPhase::Failed), 0).title, "The catalog could not be loaded");
        assert_eq!(catalog(&status(CatalogPhase::Ready), 0).title, "The catalog lists no projects");
    }

    #[test]
    fn a_catalog_with_projects_that_shows_none_is_a_filter_or_a_search() {
        for phase in [CatalogPhase::Idle, CatalogPhase::Loading, CatalogPhase::Ready, CatalogPhase::Failed] {
            assert_eq!(catalog(&status(phase), 232).title, "No projects match");
        }
    }

    #[test]
    fn an_empty_library_invites_adding_and_a_filtered_one_blames_the_filter() {
        assert!(library(false).text.contains("Catalog"));
        assert_eq!(library(true).title, "No games match");
    }

    #[test]
    fn mods_say_when_nothing_has_been_built_to_list_them() {
        assert!(mods(false).text.contains("not built yet"));
        assert_eq!(mods(true).title, "No mods match");
    }
}
