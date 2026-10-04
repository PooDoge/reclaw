//! Sample data for the gallery and the headless snapshots. Not a catalog; real data comes from
//! the install/launch backend.
use reclaw_games::project::{Media, Platform, ProjectInfo, RepoHost, RepoRef};

use std::time::Duration;

use crate::{
    activity::{ActivityBoard, ActivityEvent, ActivityId, Changelog, Kind, Stage},
    model::*,
};

pub fn sample_games() -> Vec<GameEntry> {
    let g = |id, title: &'static str, project: &'static str, version: &'static str, source, status, tags: &[&'static str]| GameEntry {
        id,
        title: title.into(),
        project: project.into(),
        version: version.into(),
        source,
        status,
        tags: tags.iter().map(|t| (*t).into()).collect(),
        // The sample library names the system in its tags, as an app added by hand would.
        platform: tags.iter().find_map(|t| Platform::from_tag(t)).unwrap_or(Platform::Other),
        art: Art::default(),
        run: RunState::Idle,
    };
    vec![
        g(1, "Starfall 64", "N64Recomp", "v1.4.2", Source::GitHub, AppStatus::Installed, &["n64"]),
        g(2, "Skyward Quest", "Zelda-style port", "v0.9.1", Source::GitLab, AppStatus::UpdateReady, &["n64", "mods"]),
        g(3, "Kart Ruins", "N64Recomp", "v0.3.0", Source::GitHub, AppStatus::NeedsFile, &["n64"]),
        g(4, "Dino Rush", "PS2 recomp", "", Source::GitHub, AppStatus::Available, &["ps2"]),
        g(5, "Moon Garden", "GBA recomp", "v2.0.0", Source::GitHub, AppStatus::Failed, &["gba"]),
        g(6, "Tide Racer", "N64Recomp", "v1.0.0", Source::GitHub, AppStatus::Installed, &["n64"]),
    ]
}

/// Two updates (one downloading, one failed) and a mod downloading, as the host would report them.
pub fn sample_activity() -> ActivityBoard {
    let mut board = ActivityBoard::new();
    let events = [
        ActivityEvent::Started {
            id: 1,
            game_id: 2,
            kind: Kind::Update,
            title: "Skyward Quest v0.9.2".into(),
            bytes_total: Some(61_000_000),
        },
        ActivityEvent::Progress {
            id: 1,
            stage: Stage::Downloading,
            bytes_done: 21_000_000,
            bytes_total: Some(61_000_000),
            rate: Some(7_400_000),
        },
        ActivityEvent::Started { id: 2, game_id: 5, kind: Kind::Update, title: "Moon Garden v2.0.1".into(), bytes_total: None },
        ActivityEvent::Failed { id: 2, reason: "Release asset not found. Check the repository or choose another version.".into() },
        ActivityEvent::Started {
            id: 3,
            game_id: 6,
            kind: Kind::Mod { provider: ModProvider::Thunderstore, id: "ghost-data".into() },
            title: "Ghost Data Pack".into(),
            bytes_total: Some(2_000_000),
        },
        ActivityEvent::Progress {
            id: 3,
            stage: Stage::Downloading,
            bytes_done: 800_000,
            bytes_total: Some(2_000_000),
            rate: Some(500_000),
        },
    ];
    for event in events {
        board.apply(event);
    }
    board
}

pub fn sample_projects() -> Vec<reclaw_games::project::ProjectInfo> {
    reclaw_games::sample::sample_projects()
}

/// Stands in for the installer in a host that has none: the events one install or update would report, each
/// with the pause before it, taking about eleven seconds. A download that fills a bar, a short verify, a
/// build whose size is not known (so the bar slides), then finished; an update ends with a changelog.
/// The pause is the host's to wait out, so this stays pure and testable.
pub fn pretend_job(id: ActivityId, game_id: u32, kind: Kind, title: &str) -> Vec<(Duration, ActivityEvent)> {
    const TOTAL: u64 = 80_000_000;
    let ms = Duration::from_millis;
    let mut events =
        vec![(ms(0), ActivityEvent::Started { id, game_id, kind: kind.clone(), title: title.into(), bytes_total: Some(TOTAL) })];
    for step in 1..=16u64 {
        let stage = Stage::Downloading;
        events.push((
            ms(500),
            ActivityEvent::Progress { id, stage, bytes_done: TOTAL / 16 * step, bytes_total: Some(TOTAL), rate: Some(10_000_000) },
        ));
    }
    events
        .push((ms(700), ActivityEvent::Progress { id, stage: Stage::Verifying, bytes_done: TOTAL, bytes_total: Some(TOTAL), rate: None }));
    for _ in 0..3 {
        events.push((ms(700), ActivityEvent::Progress { id, stage: Stage::Building, bytes_done: TOTAL, bytes_total: None, rate: None }));
    }
    let changelog = matches!(kind, Kind::Update).then(|| Changelog {
        from: "v0.9.1".into(),
        to: "v0.9.2".into(),
        notes: "Fixes the pause menu freezing on Linux.\nFixes rumble on some pads.\nAdds a borderless window mode.".into(),
        url: None,
    });
    events.push((ms(500), ActivityEvent::Finished { id, changelog }));
    events
}

/// The environment switch that points two sample projects at real public repositories.
pub const LIVE_SAMPLE: &str = "RECLAW_LIVE_SAMPLE";

/// With `RECLAW_LIVE_SAMPLE=1`, the sample games and projects numbered 1 and 3 take a real GitHub repository,
/// picture and README, so artwork, screenshots and the README on the game page can be tried against the real
/// internet. Otherwise nothing changes: the samples' `catalog://` pictures never load, which is the
/// placeholder look the snapshots rely on.
///
/// The game's own `art` is set too, because that is what the Library, the capsule grid and Deck's tiles
/// draw; the project's addresses are for the game page. The install backend will fill both from the catalog.
pub fn live_if_asked(env: impl Fn(&str) -> Option<String>, games: &mut [GameEntry], projects: &mut [ProjectInfo]) {
    if env(LIVE_SAMPLE).as_deref() != Some("1") {
        return;
    }
    let raw = |owner: &str, name: &str, path: &str| format!("https://raw.githubusercontent.com/{owner}/{name}/HEAD/{path}");
    for project in projects.iter_mut() {
        let (owner, name) = match project.id {
            1 => ("Zelda64Recomp", "Zelda64Recomp"),
            3 => ("marc2332", "freya"),
            _ => continue,
        };
        project.repo = RepoRef { host: RepoHost::Github, owner: owner.into(), name: name.into() };
        if project.id == 1 {
            // A PNG capsule and a JPEG hero that is also the first screenshot.
            project.capsule_url = Some(raw(owner, name, "icons/512.png"));
            let shot = raw(owner, name, "docs/deck_gyro_1.jpg");
            project.hero_url = Some(shot.clone());
            project.media.insert(0, Media::Screenshot { url: shot, caption: Some("Controller settings on a Steam Deck".into()) });
        } else {
            // A small PNG, scaled up to the capsule. (An SVG that uses a mask drew nothing at all, so the live
            // sample keeps to rasters, which is also what a catalog will mostly carry.)
            project.capsule_url = Some(raw("rust-lang", "rust", "src/librustdoc/html/static/images/favicon-32x32.png"));
        }
        if let Some(game) = games.iter_mut().find(|g| g.id == project.id) {
            game.art = Art { capsule: project.capsule_url.clone(), hero: project.hero_url.clone() };
        }
    }
}

pub fn sample_mods() -> Vec<ModEntry> {
    let mod_ = |provider, id: &str, game_id, title: &str, author: &str, summary: &str, version: &str, downloads, status| ModEntry {
        provider,
        id: id.into(),
        game_id,
        title: title.into(),
        author: author.into(),
        summary: summary.into(),
        version: version.into(),
        downloads,
        tags: vec![],
        status,
    };
    vec![
        mod_(
            ModProvider::Thunderstore,
            "hd-textures",
            1,
            "HD Texture Pack",
            "pixelwright",
            "Replaces the hub world textures with 4x versions.",
            "2.1.0",
            48_210,
            ModStatus::Installed,
        ),
        mod_(
            ModProvider::Thunderstore,
            "free-camera",
            1,
            "Free Camera",
            "lookaround",
            "Adds a free camera with a photo mode.",
            "1.0.3",
            12_904,
            ModStatus::Available,
        ),
        mod_(
            ModProvider::GameBanana,
            "randomizer",
            2,
            "Randomizer",
            "shuffler",
            "Shuffles item locations for a new run every time.",
            "0.8.0",
            30_555,
            ModStatus::Available,
        ),
        mod_(
            ModProvider::GameBanana,
            "hard-mode",
            2,
            "Hard Mode",
            "ironman",
            "Enemies hit harder and hearts are rarer.",
            "1.2.0",
            8_120,
            ModStatus::Available,
        ),
        mod_(
            ModProvider::Thunderstore,
            "ghost-data",
            6,
            "Ghost Data Pack",
            "ghostbusters",
            "Ten community ghosts to race against.",
            "1.0.0",
            2_033,
            ModStatus::Installing,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use reclaw_games::sample::sample_projects;
    use reclaw_media::{MediaUrl, readme::ReadmeContext};

    use super::*;

    fn env(on: bool) -> impl Fn(&str) -> Option<String> {
        move |k| (on && k == LIVE_SAMPLE).then(|| "1".to_string())
    }

    fn live(on: bool) -> (Vec<GameEntry>, Vec<ProjectInfo>) {
        let (mut games, mut projects) = (sample_games(), sample_projects());
        live_if_asked(env(on), &mut games, &mut projects);
        (games, projects)
    }

    #[test]
    fn without_the_switch_the_samples_come_back_untouched() {
        assert!(live(false) == (sample_games(), sample_projects()));
        let (mut games, mut projects) = (sample_games(), sample_projects());
        live_if_asked(|k| (k == LIVE_SAMPLE).then(|| "yes".to_string()), &mut games, &mut projects);
        assert!((games, projects) == (sample_games(), sample_projects()), "only 1 turns it on");
    }

    #[test]
    fn with_the_switch_two_games_and_projects_point_at_addresses_the_media_policy_allows() {
        let (before_games, before_projects) = (sample_games(), sample_projects());
        let (games, projects) = live(true);
        assert_eq!(projects.iter().zip(&before_projects).filter(|(a, b)| a != b).map(|(a, _)| a.id).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(games.iter().zip(&before_games).filter(|(a, b)| a != b).map(|(a, _)| a.id).collect::<Vec<_>>(), [1, 3]);

        for project in projects.iter().filter(|p| p.id == 1 || p.id == 3) {
            let context = ReadmeContext::github(&project.repo.owner, &project.repo.name, "HEAD").expect("a valid repository");
            MediaUrl::check(context.readme_url()).expect("the README address passes the policy");

            // What the switch set: the capsule, the hero, and a screenshot ahead of the sample's placeholders.
            let live = |url: &&String| url.starts_with("https://raw.githubusercontent.com/");
            let shots = project.media.iter().filter_map(|m| match m {
                Media::Screenshot { url, .. } => Some(url),
                Media::Video { .. } => None,
            });
            let art: Vec<&String> = project.capsule_url.iter().chain(&project.hero_url).chain(shots).filter(live).collect();
            assert!(!art.is_empty(), "project {} has live artwork", project.id);
            for url in art {
                MediaUrl::parse(url).unwrap_or_else(|e| panic!("{url} is refused: {e:?}"));
            }

            // The Library, the grid and Deck draw the game's art, so it must match the project's.
            let game = games.iter().find(|g| g.id == project.id).expect("a game for the project");
            assert_eq!(game.art.capsule, project.capsule_url);
            assert_eq!(game.art.hero, project.hero_url);
        }
        // The first project keeps the rest of its placeholder screenshots behind the live one.
        assert!(matches!(&projects[0].media[0], Media::Screenshot { url, .. } if url.starts_with("https://")));
        assert!(projects[0].media.len() > before_projects[0].media.len());
    }

    #[test]
    fn a_pretend_job_runs_a_believable_course_and_ends_with_a_changelog_for_updates() {
        for (kind, changelog) in [(Kind::Update, true), (Kind::Install, false)] {
            let events = pretend_job(9, 4, kind, "Dino Rush");
            let total: Duration = events.iter().map(|(wait, _)| *wait).sum();
            assert!(
                (Duration::from_secs(8)..Duration::from_secs(20)).contains(&total),
                "long enough to watch, short enough to wait: {total:?}"
            );

            let mut board = ActivityBoard::new();
            let mut last_done = 0;
            for (_, event) in events {
                board.apply(event).expect("the board accepts every event of the course");
                let activity = board.get(9).expect("the job is on the board");
                assert!(activity.bytes_done >= last_done, "progress never goes backwards");
                last_done = activity.bytes_done;
            }
            let done = board.get(9).expect("still listed once finished");
            assert_eq!(done.outcome, crate::activity::Outcome::Finished);
            assert_eq!(done.changelog.is_some(), changelog);
        }
    }
}
