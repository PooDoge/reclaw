//! Projects for tests: a handful of invented games whose declared capabilities cover every way a launch setting can be applied
//! (command line, environment, a config file in each format). Compiled only for tests (and for the crates' tests, with the
//! `fixtures` feature). The program has no sample data: its catalog is the real one (`reclaw-sync`). Ids match
//! `reclaw_ui::fixtures::sample_games`.
use crate::{
    project::{Media, Platform, ProjectInfo, Release, RepoHost, RepoRef, Requirements, SpecSheet},
    settings::{
        Base, Binding, Capabilities, ConfigFormat, ConfigPath, Constraint, Output, SettingKey, SettingValue, Size, Target, ValueType,
    },
};
use std::collections::BTreeMap;

fn repo(owner: &str, name: &str) -> RepoRef {
    RepoRef { host: RepoHost::Github, owner: owner.into(), name: name.into() }
}

fn shot(n: u32, caption: &str) -> Media {
    Media::Screenshot { url: format!("catalog://shots/{n}.png"), caption: Some(caption.into()) }
}

fn release(tag: &str, name: &str, date: &str, notes: &str, pre: bool) -> Release {
    Release {
        tag: tag.into(),
        name: name.into(),
        published: date.into(),
        notes: notes.into(),
        url: format!("https://github.com/example/releases/tag/{tag}"),
        prerelease: pre,
    }
}

fn arg(args: &[&str]) -> Output {
    Output { when: None, map: BTreeMap::new(), target: Target::Args { args: args.iter().map(|a| a.to_string()).collect() } }
}

fn arg_when(when: SettingValue, args: &[&str]) -> Output {
    Output { when: Some(when), ..arg(args) }
}

fn json_config(path: &str, value: &str, value_type: ValueType) -> Output {
    Output {
        when: None,
        map: BTreeMap::new(),
        target: Target::Config {
            file: ConfigPath { base: Base::Config, relative: "graphics.json".into() },
            format: ConfigFormat::Json,
            path: path.into(),
            value: value.into(),
            value_type,
        },
    }
}

fn bind(key: SettingKey, constraint: Constraint, outputs: Vec<Output>) -> Binding {
    Binding { key, constraint, outputs }
}

/// A game that takes everything on the command line.
fn cli_capabilities() -> Capabilities {
    Capabilities {
        settings: vec![
            bind(
                SettingKey::WindowMode,
                Constraint::None,
                vec![
                    arg_when(SettingValue::choice("exclusive"), &["--fullscreen"]),
                    arg_when(SettingValue::choice("borderless"), &["--borderless"]),
                    arg_when(SettingValue::choice("windowed"), &["--windowed"]),
                ],
            ),
            bind(SettingKey::Resolution, Constraint::None, vec![arg(&["--width", "{width}", "--height", "{height}"])]),
            bind(SettingKey::Vsync, Constraint::None, vec![arg_when(SettingValue::Bool(false), &["--no-vsync"])]),
            bind(SettingKey::FrameLimit, Constraint::Range { min: 0, max: 240 }, vec![arg(&["--fps", "{value}"])]),
        ],
    }
}

/// A game that reads a JSON config and offers a few presets.
fn json_capabilities() -> Capabilities {
    Capabilities {
        settings: vec![
            bind(
                SettingKey::Resolution,
                Constraint::Sizes(vec![Size::new(1280, 720), Size::new(1920, 1080), Size::new(2560, 1440), Size::new(3840, 2160)]),
                vec![json_config("Graphics.Width", "{width}", ValueType::Int), json_config("Graphics.Height", "{height}", ValueType::Int)],
            ),
            bind(
                SettingKey::AspectRatio,
                Constraint::Choices(vec!["auto".into(), "4:3".into(), "16:9".into()]),
                vec![json_config("Graphics.AspectRatio", "{value}", ValueType::String)],
            ),
            bind(
                SettingKey::Msaa,
                Constraint::Choices(vec!["off".into(), "2x".into(), "4x".into()]),
                vec![json_config("Graphics.MSAA", "{value}", ValueType::String)],
            ),
            bind(
                SettingKey::UpscaleMethod,
                Constraint::Choices(vec!["off".into(), "fsr1".into()]),
                vec![json_config("Graphics.Upscaler", "{value}", ValueType::String)],
            ),
        ],
    }
}

pub fn sample_projects() -> Vec<ProjectInfo> {
    let sheet = |os: &str, cpu: &str, gpu: &str, mem: &str, disk: &str| SpecSheet {
        os: Some(os.into()),
        cpu: Some(cpu.into()),
        gpu: Some(gpu.into()),
        memory: Some(mem.into()),
        storage: Some(disk.into()),
    };
    let base = |id: u32, title: &str, platform: Platform, owner: &str, name: &str, summary: &str| ProjectInfo {
        id,
        title: title.into(),
        project: String::new(),
        summary: summary.into(),
        description: format!("{summary} Built from your own copy of the original game; nothing copyrighted is downloaded."),
        platform,
        repo: repo(owner, name),
        hero_url: Some(format!("catalog://hero/{id}.jpg")),
        capsule_url: Some(format!("catalog://capsule/{id}.jpg")),
        media: vec![],
        releases: vec![],
        requirements: None,
        tags: vec![],
        capabilities: Capabilities::default(),
        listing: None,
    };
    vec![
        ProjectInfo {
            media: vec![
                shot(1, "Hub world at 1440p"),
                shot(2, "Widescreen with the HD texture pack"),
                shot(3, "The in-game options menu"),
                Media::Video {
                    url: "https://example.com/starfall64-trailer".into(),
                    thumbnail: Some("catalog://shots/trailer.png".into()),
                    title: Some("Launch trailer".into()),
                },
            ],
            releases: vec![
                release("v1.4.2", "Controller fixes", "2026-09-28", "Fixes rumble on Linux pads.\nFixes the ultrawide camera cut.", false),
                release("v1.4.0", "Ultrawide", "2026-09-02", "Adds 21:9 and 32:9 support.\nNew frame pacing.", false),
                release("v1.5.0-rc1", "Next: mod loader", "2026-09-30", "Preview of the mod loader.", true),
            ],
            requirements: Some(Requirements {
                minimum: sheet("Linux or Windows 10", "Dual core 2.5 GHz", "OpenGL 3.3 / Vulkan 1.1", "4 GB", "1.2 GB"),
                recommended: Some(sheet("Linux or Windows 11", "Quad core 3 GHz", "Vulkan 1.3 GPU", "8 GB", "1.2 GB")),
                notes: Some("Needs your own Starfall 64 ROM (US).".into()),
            }),
            tags: vec!["n64".into(), "widescreen".into()],
            capabilities: cli_capabilities(),
            ..base(
                1,
                "Starfall 64",
                Platform::N64,
                "starfall-recomp",
                "Starfall64Recomp",
                "A 3D platformer recompiled for modern hardware.",
            )
        },
        ProjectInfo {
            media: vec![shot(4, "Overworld"), shot(5, "Dungeon")],
            releases: vec![
                release("v0.9.2", "Save fixes", "2026-09-20", "Fixes a save corruption bug.", false),
                release("v0.9.1", "First public build", "2026-08-30", "Initial release.", false),
            ],
            requirements: Some(Requirements {
                minimum: sheet("Linux or Windows 10", "Quad core", "Vulkan 1.1", "8 GB", "2 GB"),
                recommended: None,
                notes: None,
            }),
            tags: vec!["n64".into(), "mods".into()],
            capabilities: json_capabilities(),
            ..base(2, "Skyward Quest", Platform::N64, "skyward-port", "SkywardRecomp", "An adventure port with mod support.")
        },
        ProjectInfo {
            releases: vec![release("v0.3.0", "Alpha", "2026-09-10", "Alpha build.", true)],
            tags: vec!["n64".into()],
            ..base(3, "Kart Ruins", Platform::N64, "kart-recomp", "KartRuins", "Kart racing on the original tracks.")
        },
        ProjectInfo {
            releases: vec![],
            tags: vec!["ps2".into()],
            ..base(4, "Dino Rush", Platform::Ps2, "dinorush", "DinoRushRecomp", "A PS2 racer, early days.")
        },
        ProjectInfo {
            releases: vec![release("v2.0.1", "Hotfix", "2026-09-27", "Release asset layout changed.", false)],
            tags: vec!["gba".into()],
            ..base(5, "Moon Garden", Platform::Gba, "moongarden", "MoonGardenRecomp", "A GBA puzzle adventure.")
        },
        ProjectInfo {
            releases: vec![release("v1.0.0", "1.0", "2026-08-12", "First stable release.", false)],
            tags: vec!["n64".into()],
            capabilities: cli_capabilities(),
            ..base(6, "Tide Racer", Platform::N64, "tide-racer", "TideRacer", "Boat racing with online-style ghosts.")
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_some_games_declare_no_settings() {
        let projects = sample_projects();
        let mut ids: Vec<u32> = projects.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), projects.len());
        assert!(projects.iter().any(|p| p.capabilities.is_empty()), "the page must be shown for a game with nothing to configure");
        assert!(projects.iter().any(|p| !p.capabilities.is_empty()));
    }

    #[test]
    fn sample_capabilities_round_trip_through_json() {
        for p in sample_projects() {
            let json = serde_json::to_string(&p.capabilities).expect("serialize");
            let back: Capabilities = serde_json::from_str(&json).expect("parse");
            assert_eq!(back, p.capabilities, "{}", p.title);
        }
    }
}
