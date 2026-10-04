//! Browsing by system: which systems a list of games covers, narrowing to one, and the sort orders.
//! Pure, so the Library, the Catalog and Deck mode order and filter the same way.
use reclaw_games::project::Platform;

use crate::{
    model::GameEntry,
    settings::{KEY_LIBRARY_SORT, SettingsTarget, SettingsValues},
};

/// How a list of games is ordered.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Sort {
    /// The order games joined the library: how the host lists them. What a fresh install shows.
    #[default]
    Added,
    /// A to Z.
    Title,
    /// Grouped by system (Nintendo's together, then Sony's, and so on, oldest first), A to Z within each.
    System,
}

impl Sort {
    /// In the order the settings row lists them, which is also the order saved by position.
    pub const ALL: [Sort; 3] = [Sort::Added, Sort::Title, Sort::System];

    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Title => "Title",
            Self::System => "System",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// A position out of range (a file from a newer version) is the default.
    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or_default()
    }

    /// From the Library section's sort row. Untouched is by title.
    pub fn from_settings(values: &SettingsValues) -> Self {
        Self::from_index(values.choice(SettingsTarget::Global, KEY_LIBRARY_SORT, 0))
    }
}

/// `games` in `sort` order. Stable, so games that compare equal keep the order they came in.
pub fn sorted(mut games: Vec<GameEntry>, sort: Sort) -> Vec<GameEntry> {
    let title = |g: &GameEntry| g.title.to_lowercase();
    match sort {
        Sort::Added => {}
        Sort::Title => games.sort_by_cached_key(title),
        Sort::System => games.sort_by_cached_key(|g| (g.platform.rank(), title(g))),
    }
    games
}

/// The games on `system`, or all of them for `None`.
pub fn on_system(games: Vec<GameEntry>, system: Option<Platform>) -> Vec<GameEntry> {
    games.into_iter().filter(|g| system.is_none_or(|wanted| g.platform == wanted)).collect()
}

/// The systems `games` cover with how many each, in system order: what the filter offers.
pub fn systems_in(games: &[GameEntry]) -> Vec<(Platform, u32)> {
    Platform::ALL
        .into_iter()
        .filter_map(|platform| {
            let count = games.iter().filter(|g| g.platform == platform).count() as u32;
            (count > 0).then_some((platform, count))
        })
        .collect()
}

/// The games split into one run per system, in system order, for a list that shows a heading for each.
pub fn grouped(games: &[GameEntry]) -> Vec<(Platform, Vec<GameEntry>)> {
    systems_in(games)
        .into_iter()
        .map(|(platform, _)| (platform, sorted(games.iter().filter(|g| g.platform == platform).cloned().collect(), Sort::Title)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample_games;

    fn titles(games: &[GameEntry]) -> Vec<&str> {
        games.iter().map(|g| &*g.title).collect()
    }

    #[test]
    fn added_leaves_the_order_alone() {
        assert_eq!(titles(&sorted(sample_games(), Sort::Added)), titles(&sample_games()));
    }

    #[test]
    fn by_title_is_alphabetical_ignoring_case() {
        assert_eq!(
            titles(&sorted(sample_games(), Sort::Title)),
            ["Dino Rush", "Kart Ruins", "Moon Garden", "Skyward Quest", "Starfall 64", "Tide Racer"]
        );
    }

    #[test]
    fn by_system_groups_oldest_first_then_alphabetical() {
        // The sample has four N64 games, a GBA game and a PS2 game.
        assert_eq!(
            titles(&sorted(sample_games(), Sort::System)),
            ["Kart Ruins", "Skyward Quest", "Starfall 64", "Tide Racer", "Moon Garden", "Dino Rush"]
        );
    }

    #[test]
    fn narrowing_to_a_system_keeps_only_it() {
        assert_eq!(titles(&on_system(sample_games(), Some(Platform::Ps2))), ["Dino Rush"]);
        assert_eq!(on_system(sample_games(), None).len(), sample_games().len());
        assert!(on_system(sample_games(), Some(Platform::Dreamcast)).is_empty());
    }

    #[test]
    fn the_systems_a_list_covers_are_counted_in_system_order() {
        assert_eq!(systems_in(&sample_games()), [(Platform::N64, 4), (Platform::Gba, 1), (Platform::Ps2, 1)]);
        assert!(systems_in(&[]).is_empty());
    }

    #[test]
    fn grouping_gives_a_run_per_system() {
        let groups = grouped(&sample_games());
        assert_eq!(groups.iter().map(|(p, _)| *p).collect::<Vec<_>>(), [Platform::N64, Platform::Gba, Platform::Ps2]);
        assert_eq!(titles(&groups[0].1), ["Kart Ruins", "Skyward Quest", "Starfall 64", "Tide Racer"]);
    }

    #[test]
    fn the_sort_row_maps_to_a_sort_and_back() {
        for sort in Sort::ALL {
            assert_eq!(Sort::from_index(sort.index()), sort);
        }
        assert_eq!(Sort::from_index(9), Sort::Added, "an unknown position is the default");
        assert_eq!(Sort::from_settings(&SettingsValues::default()), Sort::Added);
    }
}
