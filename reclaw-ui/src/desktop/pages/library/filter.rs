//! Which games the Library lists, and in what order: the filter chips, the system filter, the
//! search box and the sort.
use reclaw_games::project::Platform;

use crate::{
    catalog::matches_query,
    prelude::*,
    systems::{Sort, on_system, sorted},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Installed,
    Updates,
    Favorites,
}

impl Filter {
    pub fn matches(self, game: &GameEntry) -> bool {
        match self {
            Self::All => true,
            Self::Installed => game.status.is_installed(),
            Self::Updates => game.status == AppStatus::UpdateReady,
            Self::Favorites => game.is_favorite(),
        }
    }
}

/// The games that pass `filter`, are on `system` (or any, for `None`) and whose title, project or any
/// tag contains `query` (case-insensitive; an empty query matches everything), in `sort` order.
pub fn visible(games: &[GameEntry], filter: Filter, system: Option<Platform>, query: &str, sort: Sort) -> Vec<GameEntry> {
    let passing = games.iter().filter(|g| filter.matches(g)).filter(|g| matches_query(g, query)).cloned().collect();
    sorted(on_system(passing, system), sort)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::sample_games;

    fn titles(v: &[GameEntry]) -> Vec<&str> {
        v.iter().map(|g| &*g.title).collect()
    }

    #[test]
    fn all_with_no_query_lists_everything() {
        assert_eq!(visible(&sample_games(), Filter::All, None, "", Sort::Title).len(), sample_games().len());
    }

    #[test]
    fn installed_and_updates_filter_by_status() {
        let games = sample_games();
        assert!(visible(&games, Filter::Installed, None, "", Sort::Title).iter().all(|g| g.status.is_installed()));
        assert_eq!(titles(&visible(&games, Filter::Updates, None, "", Sort::Title)), vec!["Skyward Quest"]);
    }

    #[test]
    fn search_matches_title_project_and_tags_ignoring_case() {
        let games = sample_games();
        assert_eq!(titles(&visible(&games, Filter::All, None, "DINO", Sort::Title)), vec!["Dino Rush"]);
        assert_eq!(titles(&visible(&games, Filter::All, None, "ps2 recomp", Sort::Title)), vec!["Dino Rush"]);
        assert_eq!(titles(&visible(&games, Filter::All, None, "mods", Sort::Title)), vec!["Skyward Quest"]);
    }

    #[test]
    fn filter_and_search_combine() {
        assert!(visible(&sample_games(), Filter::Updates, None, "dino", Sort::Title).is_empty());
    }

    #[test]
    fn the_system_filter_and_the_sort_apply_after_the_others() {
        let games = sample_games();
        assert_eq!(
            titles(&visible(&games, Filter::All, Some(Platform::N64), "", Sort::Title)),
            vec!["Kart Ruins", "Skyward Quest", "Starfall 64", "Tide Racer"]
        );
        assert_eq!(
            titles(&visible(&games, Filter::Installed, Some(Platform::N64), "", Sort::Title)),
            vec!["Skyward Quest", "Starfall 64", "Tide Racer"],
            "a game with an update ready is installed"
        );
        assert!(visible(&games, Filter::All, Some(Platform::Ps2), "starfall", Sort::Title).is_empty());
        let by_system = visible(&games, Filter::All, None, "", Sort::System);
        assert_eq!(by_system.first().map(|g| g.platform), Some(Platform::N64));
        assert_eq!(by_system.last().map(|g| g.platform), Some(Platform::Ps2));
    }
}
