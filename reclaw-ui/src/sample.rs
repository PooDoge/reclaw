//! Sample data for the gallery and the headless snapshots. Not a catalog; real data comes from
//! the install/launch backend.
use crate::model::*;

pub fn sample_games() -> Vec<GameEntry> {
    let g = |id, title: &'static str, project: &'static str, version: &'static str, source, status, tags: &[&'static str]| GameEntry {
        id,
        title: title.into(),
        project: project.into(),
        version: version.into(),
        source,
        status,
        tags: tags.iter().map(|t| (*t).into()).collect(),
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

pub fn sample_downloads() -> Vec<Download> {
    vec![
        Download {
            app_id: 2,
            title: "Skyward Quest v0.9.2".into(),
            stage: DownloadStage::FetchingRelease,
            progress: 34.,
            detail: "21 MB of 61 MB".into(),
            speed: Some("7.4 MB/s".into()),
            error: None,
        },
        Download {
            app_id: 5,
            title: "Moon Garden v2.0.1".into(),
            stage: DownloadStage::FetchingRelease,
            progress: 48.,
            detail: "".into(),
            speed: None,
            error: Some("Release asset not found. Check the repository or choose another version.".into()),
        },
    ]
}

pub fn sample_projects() -> Vec<reclaw_games::project::ProjectInfo> {
    reclaw_games::sample::sample_projects()
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
