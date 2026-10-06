//! Invented games, projects, mods and activity for tests, so the pure rules and the headless UI tests have something to work on
//! that never changes and needs no network. Compiled only for tests (and for the integration tests, with the `fixtures`
//! feature). The program itself has no sample data: it shows the real catalog (`reclaw-sync`) and the user's own library.
use reclaw_games::project::Platform;

use crate::{
    activity::{ActivityBoard, ActivityEvent, Kind, Stage},
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
        platform: Platform::from_tags(tags.iter()).unwrap_or(Platform::Other),
        art: Art::default(),
        in_library: true,
        run: RunState::Idle,
    };
    vec![
        g(1, "Starfall 64", "N64Recomp", "v1.4.2", Source::GitHub, AppStatus::Installed, &["n64"]),
        g(2, "Skyward Quest", "Zelda-style port", "v0.9.1", Source::GitLab, AppStatus::UpdateReady, &["n64", "mods"]),
        g(3, "Kart Ruins", "N64Recomp", "v0.3.0", Source::GitHub, AppStatus::Available, &["n64"]),
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
        ActivityEvent::Failed {
            id: 2,
            reason: "Release asset not found. Check the repository or choose another version.".into(),
            details: Vec::new(),
        },
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
    reclaw_games::fixtures::sample_projects()
}

fn status_is_installed(status: ModStatus) -> bool {
    status.is_installed()
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
        installed_version: status_is_installed(status).then(|| version.into()),
        downloads,
        tags: vec![],
        status,
        icon: None,
        page_url: Some(format!("https://example.invalid/mods/{id}")),
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

use crate::store::AppState;

impl AppState {
    /// A state with the fixtures and no saved settings.
    pub fn sample() -> Self {
        Self { games: sample_games(), projects: sample_projects(), mods: sample_mods(), activity: sample_activity(), ..Self::default() }
    }
}
