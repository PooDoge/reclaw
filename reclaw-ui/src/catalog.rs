//! Joining what the user has (the library) with what the catalog says (projects): the data behind
//! the Game page and the Catalog. Pure, so both interfaces share it and it is tested without a window.
use reclaw_games::{
    project::{Platform, ProjectInfo},
    settings::{DisplayEnvironment, SettingSpec, supported},
};

use crate::model::{GameEntry, ModEntry, ModProvider};

/// Everything the Game page shows about one game.
#[derive(Clone, PartialEq, Debug)]
pub struct GameView {
    /// The library entry, or one made from the project when the user has not added it.
    pub game: GameEntry,
    /// What the catalog knows. `None` for an app added by hand.
    pub project: Option<ProjectInfo>,
    /// Mods made for this game.
    pub mods: Vec<ModEntry>,
}

impl GameView {
    /// The view for a game id, from the library or, failing that, the catalog. `None` when neither
    /// knows the id.
    pub fn resolve(id: u32, games: &[GameEntry], projects: &[ProjectInfo], mods: &[ModEntry]) -> Option<Self> {
        let project = projects.iter().find(|p| p.id == id).cloned();
        let game = games.iter().find(|g| g.id == id).cloned().or_else(|| project.as_ref().map(GameEntry::from_project))?;
        Some(Self { game, project, mods: mods.iter().filter(|m| m.game_id == id).cloned().collect() })
    }

    /// The launch settings this game honors on this display. Empty when the game declares none, or
    /// the project is unknown: then no settings are offered for it anywhere.
    pub fn launch_settings(&self, env: &DisplayEnvironment) -> Vec<SettingSpec> {
        self.project.as_ref().map(|p| supported(&p.capabilities, env)).unwrap_or_default()
    }
}

/// Every project that passes the platform filter and the search, as a library entry when the user
/// has added it and as an available one otherwise.
pub fn catalog_entries(games: &[GameEntry], projects: &[ProjectInfo], platform: Option<Platform>, query: &str) -> Vec<GameEntry> {
    projects
        .iter()
        .filter(|p| platform.is_none_or(|wanted| p.platform == wanted))
        .map(|p| games.iter().find(|g| g.id == p.id).cloned().unwrap_or_else(|| GameEntry::from_project(p)))
        .filter(|g| matches_query(g, query))
        .collect()
}

/// The platforms the catalog has projects for, in the order they first appear, with a count each.
pub fn platforms(projects: &[ProjectInfo]) -> Vec<(Platform, u32)> {
    let mut out: Vec<(Platform, u32)> = Vec::new();
    for p in projects {
        match out.iter_mut().find(|(platform, _)| *platform == p.platform) {
            Some((_, count)) => *count += 1,
            None => out.push((p.platform, 1)),
        }
    }
    out
}

/// Mods from one provider (or all) whose title, author, summary or tags contain `query`.
pub fn mods_matching(mods: &[ModEntry], provider: Option<ModProvider>, query: &str) -> Vec<ModEntry> {
    let query = query.trim().to_lowercase();
    mods.iter()
        .filter(|m| provider.is_none_or(|wanted| m.provider == wanted))
        .filter(|m| {
            query.is_empty()
                || m.title.to_lowercase().contains(&query)
                || m.author.to_lowercase().contains(&query)
                || m.summary.to_lowercase().contains(&query)
                || m.tags.iter().any(|t| t.to_lowercase().contains(&query))
        })
        .cloned()
        .collect()
}

/// Whether a title, project name or any tag contains `query`, ignoring case. An empty query matches.
pub fn matches_query(game: &GameEntry, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || game.title.to_lowercase().contains(&query)
        || game.project.to_lowercase().contains(&query)
        || game.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
}

#[cfg(test)]
mod tests {
    use reclaw_games::settings::SettingKey;

    use super::*;
    use crate::{
        model::AppStatus,
        sample::{sample_games, sample_mods, sample_projects},
    };

    #[test]
    fn a_library_game_keeps_its_own_state_and_gains_the_project() {
        let view = GameView::resolve(2, &sample_games(), &sample_projects(), &sample_mods()).expect("known");
        assert_eq!(view.game.status, AppStatus::UpdateReady);
        assert_eq!(view.project.as_ref().map(|p| p.title.as_str()), Some("Skyward Quest"));
        assert!(view.mods.iter().all(|m| m.game_id == 2) && !view.mods.is_empty());
    }

    #[test]
    fn a_project_the_user_has_not_added_is_available() {
        let view = GameView::resolve(4, &[], &sample_projects(), &[]).expect("known by the catalog");
        assert_eq!(view.game.status, AppStatus::Available);
        assert_eq!(&*view.game.title, "Dino Rush");
    }

    #[test]
    fn a_library_game_without_a_project_still_resolves() {
        let view = GameView::resolve(1, &sample_games(), &[], &[]).expect("in the library");
        assert!(view.project.is_none());
        assert!(view.launch_settings(&DisplayEnvironment::unknown()).is_empty());
    }

    #[test]
    fn an_unknown_id_resolves_to_nothing() {
        assert!(GameView::resolve(999, &sample_games(), &sample_projects(), &sample_mods()).is_none());
    }

    #[test]
    fn launch_settings_come_only_from_what_the_game_declares() {
        let env = DisplayEnvironment::unknown();
        let with = GameView::resolve(1, &sample_games(), &sample_projects(), &[]).expect("known");
        let without = GameView::resolve(3, &sample_games(), &sample_projects(), &[]).expect("known");
        let keys: Vec<SettingKey> = with.launch_settings(&env).iter().map(|s| s.key).collect();
        assert!(keys.contains(&SettingKey::WindowMode) && !keys.contains(&SettingKey::Msaa));
        assert!(without.launch_settings(&env).is_empty(), "Kart Ruins declares nothing, so nothing is shown");
    }

    #[test]
    fn the_catalog_lists_every_project_once_with_library_state_where_there_is_some() {
        let entries = catalog_entries(&sample_games(), &sample_projects(), None, "");
        assert_eq!(entries.len(), sample_projects().len());
        assert_eq!(entries.iter().find(|g| g.id == 2).map(|g| g.status), Some(AppStatus::UpdateReady));
        let only_projects = catalog_entries(&[], &sample_projects(), None, "");
        assert!(only_projects.iter().all(|g| g.status == AppStatus::Available));
    }

    #[test]
    fn the_catalog_filters_by_platform_and_search() {
        let (games, projects) = (sample_games(), sample_projects());
        let ps2 = catalog_entries(&games, &projects, Some(Platform::Ps2), "");
        assert_eq!(ps2.iter().map(|g| &*g.title).collect::<Vec<_>>(), vec!["Dino Rush"]);
        assert!(catalog_entries(&games, &projects, Some(Platform::Ps2), "starfall").is_empty());
        assert_eq!(catalog_entries(&games, &projects, None, "garden").len(), 1);
    }

    #[test]
    fn platforms_are_counted_in_first_seen_order() {
        let counts = platforms(&sample_projects());
        assert_eq!(counts.first(), Some(&(Platform::N64, 4)));
        assert_eq!(counts.iter().map(|(_, n)| n).sum::<u32>() as usize, sample_projects().len());
    }

    #[test]
    fn mods_filter_by_provider_and_text() {
        let mods = sample_mods();
        assert!(mods_matching(&mods, Some(ModProvider::GameBanana), "").iter().all(|m| m.provider == ModProvider::GameBanana));
        assert_eq!(mods_matching(&mods, None, "RANDOM").len(), 1);
        assert_eq!(mods_matching(&mods, None, "").len(), mods.len());
        assert!(mods_matching(&mods, Some(ModProvider::Thunderstore), "randomizer").is_empty());
    }

    #[test]
    fn queries_match_title_project_and_tags_ignoring_case() {
        let games = sample_games();
        assert!(matches_query(&games[0], "  STARFALL "));
        assert!(matches_query(&games[0], "n64recomp"));
        assert!(matches_query(&games[1], "mods"));
        assert!(matches_query(&games[0], ""));
        assert!(!matches_query(&games[0], "zelda"));
    }
}
