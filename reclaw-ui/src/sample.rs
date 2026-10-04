//! Sample data for the gallery and the headless snapshots. Not a catalog; real data comes from
//! the install/launch backend.
use crate::model::*;

pub fn sample_games() -> Vec<GameEntry> {
    let g = |id,
             title: &'static str,
             project: &'static str,
             version: &'static str,
             source,
             status,
             tags: &[&'static str]| GameEntry {
        id,
        title: title.into(),
        project: project.into(),
        version: version.into(),
        source,
        status,
        tags: tags.iter().map(|t| (*t).into()).collect(),
    };
    vec![
        g(
            1,
            "Starfall 64",
            "N64Recomp",
            "v1.4.2",
            Source::GitHub,
            AppStatus::Installed,
            &["n64"],
        ),
        g(
            2,
            "Skyward Quest",
            "Zelda-style port",
            "v0.9.1",
            Source::GitLab,
            AppStatus::UpdateReady,
            &["n64", "mods"],
        ),
        g(
            3,
            "Kart Ruins",
            "N64Recomp",
            "v0.3.0",
            Source::GitHub,
            AppStatus::NeedsFile,
            &["n64"],
        ),
        g(
            4,
            "Dino Rush",
            "PS2 recomp",
            "",
            Source::GitHub,
            AppStatus::Available,
            &["ps2"],
        ),
        g(
            5,
            "Moon Garden",
            "GBA recomp",
            "v2.0.0",
            Source::GitHub,
            AppStatus::Failed,
            &["gba"],
        ),
        g(
            6,
            "Tide Racer",
            "N64Recomp",
            "v1.0.0",
            Source::GitHub,
            AppStatus::Installed,
            &["n64"],
        ),
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
            error: Some(
                "Release asset not found. Check the repository or choose another version.".into(),
            ),
        },
    ]
}
