//! Which game the Mods tab is showing mods for: the installed games that take mods, each with how many
//! mods are listed for it, and the user's choice among them. Pure; the Mods tab and the Game page's
//! link draw it.
use crate::model::{GameEntry, ModEntry};

/// A game the Mods tab can narrow to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ModGame {
    pub id: u32,
    pub title: String,
    /// Mods listed for it, installed or not.
    pub mods: u32,
}

/// The library games in `moddable`, by title ignoring case. An id the library does not know (the two
/// lists arrive separately, so one can be a step behind the other) is left out rather than shown nameless.
pub fn mod_games(games: &[GameEntry], moddable: &[u32], mods: &[ModEntry]) -> Vec<ModGame> {
    let mut shown: Vec<ModGame> = moddable
        .iter()
        .filter_map(|id| games.iter().find(|g| g.id == *id))
        .map(|g| ModGame { id: g.id, title: g.title.to_string(), mods: mods.iter().filter(|m| m.game_id == g.id).count() as u32 })
        .collect();
    shown.sort_by_cached_key(|g| (g.title.to_lowercase(), g.id));
    shown.dedup_by_key(|g| g.id);
    shown
}

/// The game the tab narrows to: the one chosen, while it is still a choice. A game uninstalled since
/// it was chosen falls back to all games instead of an empty list with no way to see why.
pub fn chosen(choice: Option<u32>, games: &[ModGame]) -> Option<&ModGame> {
    choice.and_then(|id| games.iter().find(|g| g.id == id))
}

/// Whether a game's page offers a way to its mods.
pub fn takes_mods(id: u32, moddable: &[u32]) -> bool {
    moddable.contains(&id)
}

/// What the game chip says and what its list offers: "All games" first, then one line per game.
pub fn picker_labels(games: &[ModGame]) -> Vec<String> {
    std::iter::once("All games".to_string()).chain(games.iter().map(|g| format!("{} ({})", g.title, g.mods))).collect()
}

/// The choice for a line of [`picker_labels`]: line 0 is all games.
pub fn picked(index: usize, games: &[ModGame]) -> Option<u32> {
    index.checked_sub(1).and_then(|i| games.get(i)).map(|g| g.id)
}

/// The line of [`picker_labels`] the cursor starts on for `choice`.
pub fn picker_index(choice: Option<u32>, games: &[ModGame]) -> usize {
    choice.and_then(|id| games.iter().position(|g| g.id == id)).map_or(0, |i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{sample_games, sample_mods};

    fn sample() -> Vec<ModGame> {
        mod_games(&sample_games(), &[6, 1, 2], &sample_mods())
    }

    #[test]
    fn the_moddable_games_are_listed_by_title_with_their_mod_counts() {
        let games = sample();
        let titles: Vec<(&str, u32)> = games.iter().map(|g| (g.title.as_str(), g.mods)).collect();
        assert_eq!(titles, vec![("Skyward Quest", 2), ("Starfall 64", 2), ("Tide Racer", 1)]);
    }

    #[test]
    fn a_game_with_no_mods_listed_yet_is_still_a_choice() {
        let games = mod_games(&sample_games(), &[5], &sample_mods());
        assert_eq!(games, vec![ModGame { id: 5, title: "Moon Garden".into(), mods: 0 }]);
    }

    #[test]
    fn ids_the_library_does_not_know_and_repeats_are_left_out() {
        let games = mod_games(&sample_games(), &[1, 999, 1], &sample_mods());
        assert_eq!(games.iter().map(|g| g.id).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn a_choice_that_is_no_longer_offered_means_all_games() {
        let games = sample();
        assert_eq!(chosen(Some(2), &games).map(|g| g.id), Some(2));
        assert_eq!(chosen(Some(3), &games), None, "Kart Ruins is not installed");
        assert_eq!(chosen(None, &games), None);
    }

    #[test]
    fn the_picker_lines_map_to_choices_and_back() {
        let games = sample();
        assert_eq!(picker_labels(&games), vec!["All games", "Skyward Quest (2)", "Starfall 64 (2)", "Tide Racer (1)"]);
        assert_eq!(picked(0, &games), None);
        assert_eq!(picked(3, &games), Some(6));
        assert_eq!(picked(9, &games), None);
        assert_eq!(picker_index(Some(6), &games), 3);
        assert_eq!(picker_index(Some(3), &games), 0);
        assert_eq!(picker_index(None, &games), 0);
    }

    #[test]
    fn only_moddable_games_offer_their_mods() {
        assert!(takes_mods(2, &[1, 2]));
        assert!(!takes_mods(3, &[1, 2]));
    }
}
