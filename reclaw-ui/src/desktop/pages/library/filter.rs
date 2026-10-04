//! Which games the Library lists: the filter chips and the search box.
use crate::{catalog::matches_query, prelude::*};

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

/// The games that pass `filter` and whose title, project or any tag contains `query`
/// (case-insensitive; an empty query matches everything).
pub fn visible(games: &[GameEntry], filter: Filter, query: &str) -> Vec<GameEntry> {
    games.iter().filter(|g| filter.matches(g)).filter(|g| matches_query(g, query)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample_games;

    fn titles(v: &[GameEntry]) -> Vec<&str> {
        v.iter().map(|g| &*g.title).collect()
    }

    #[test]
    fn all_with_no_query_lists_everything() {
        assert_eq!(visible(&sample_games(), Filter::All, "").len(), sample_games().len());
    }

    #[test]
    fn installed_and_updates_filter_by_status() {
        let games = sample_games();
        assert!(visible(&games, Filter::Installed, "").iter().all(|g| g.status.is_installed()));
        assert_eq!(titles(&visible(&games, Filter::Updates, "")), vec!["Skyward Quest"]);
    }

    #[test]
    fn search_matches_title_project_and_tags_ignoring_case() {
        let games = sample_games();
        assert_eq!(titles(&visible(&games, Filter::All, "DINO")), vec!["Dino Rush"]);
        assert_eq!(titles(&visible(&games, Filter::All, "ps2 recomp")), vec!["Dino Rush"]);
        assert_eq!(titles(&visible(&games, Filter::All, "mods")), vec!["Skyward Quest"]);
    }

    #[test]
    fn filter_and_search_combine() {
        assert!(visible(&sample_games(), Filter::Updates, "dino").is_empty());
    }
}
