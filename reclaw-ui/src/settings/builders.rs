//! The two schemas Reclaw ships: per-app Properties and global Settings. Both interfaces show these,
//! so a row added here appears in both.
use reclaw_games::settings::{Group as LaunchGroup, SettingKey, SettingSpec};

use super::schema::*;
use crate::model::GameEntry;

pub const MODE_OPTIONS: &[&str] = &["Auto", "Desktop", "Deck"];
pub const THEME_OPTIONS: &[&str] = &["Midnight", "Daylight"];
pub const INTENSITY_OPTIONS: &[&str] = &["Off", "Subtle", "Standard", "Cinematic"];
pub const STYLE_OPTIONS: &[&str] = &["Slide", "Fade", "Rise", "Zoom"];
pub const SCALE_OPTIONS: &[&str] = &["100%", "125%", "150%"];
pub const SAFE_ZONE_OPTIONS: &[&str] = &["Off", "Small", "Large"];
pub const CONFIRM_OPTIONS: &[&str] = &["Bottom button (A, cross)", "Right button (B, circle)"];
pub const CHANNEL_OPTIONS: &[&str] = &["Stable", "Pre-release"];

/// Keys that need more than a stored value when they change.
pub const KEY_INTERFACE_MODE: &str = "interface_mode";
/// Keys other code reads to configure itself. Renaming one loses what people chose.
pub const KEY_THEME: &str = "theme";
pub const KEY_REDUCE_MOTION: &str = "motion_reduce";
pub const KEY_DESKTOP_INTENSITY: &str = "motion_desktop_intensity";
pub const KEY_DESKTOP_STYLE: &str = "motion_desktop_style";
pub const KEY_DESKTOP_PAGE_STYLES: &str = "motion_desktop_pages";
pub const KEY_CONSOLE_INTENSITY: &str = "motion_console_intensity";
pub const KEY_CONSOLE_STYLE: &str = "motion_console_style";
pub const KEY_CONSOLE_PAGE_STYLES: &str = "motion_console_pages";

fn toggle(key: &'static str, label: &'static str, default: bool) -> Row {
    Row::new(key, label, RowKind::Toggle { default })
}

fn choice(key: &'static str, label: &'static str, options: &'static [&'static str], default: usize) -> Row {
    Row::new(key, label, RowKind::Choice { options, default })
}

/// The launch settings as a section: one group per kind (Display, Upscaling, Graphics), only the
/// groups that have rows. `None` when the game, or the display, offers nothing to set.
fn launch_section(id: &'static str, title: &'static str, specs: &[SettingSpec]) -> Option<Section> {
    let groups: Vec<Group> = LaunchGroup::ALL
        .into_iter()
        .filter_map(|group| {
            let rows: Vec<Row> = specs.iter().filter(|spec| spec.key.group() == group).map(|spec| launch_row(spec.key)).collect();
            (!rows.is_empty()).then(|| Group::new(rows).headed(group.label()))
        })
        .collect();
    (!groups.is_empty()).then_some(Section { id, title, groups })
}

fn launch_row(key: SettingKey) -> Row {
    Row::new(key.id(), key.label(), RowKind::Launch { key }).described(key.description())
}

/// Properties of one installed or installable app (the "Properties..." page). `launch` is what the
/// game and the display agree on (`GameView::launch_settings`); a game that offers nothing has no
/// Display section at all, rather than one that does nothing.
pub fn app_properties(game: &GameEntry, launch: &[SettingSpec]) -> Schema {
    let mut schema = app_sections(game);
    if let Some(section) = launch_section("display", "Display and graphics", launch) {
        schema.sections.insert(1, section);
    }
    schema
}

fn app_sections(game: &GameEntry) -> Schema {
    Schema {
        title: game.title.to_string(),
        sections: vec![
            Section {
                id: "general",
                title: "General",
                groups: vec![
                    Group::new(vec![toggle("keep_prerelease", "Keep pre-release builds", false)]),
                    Group::new(vec![
                        Row::new(
                            "launch_options",
                            "Launch options",
                            RowKind::Text { field: TextField::LaunchOptions, placeholder: "Arguments passed to the app" },
                        )
                        .described("For advanced users: extra command-line arguments."),
                    ])
                    .headed("Launch options"),
                ],
            },
            Section {
                id: "controller",
                title: "Controller",
                groups: vec![
                    Group::new(vec![
                        toggle("swap_ab", "Swap A and B for this app", false)
                            .described("Rewrites the app's SDL controller mapping; apps that read SDL pick it up."),
                        toggle("background_events", "Let the app read the pad when it is not focused", true),
                    ]),
                    Group::new(vec![Row::new(
                        "sdl_override",
                        "SDL mapping override",
                        RowKind::Text { field: TextField::SdlOverride, placeholder: "A full SDL_GAMECONTROLLERCONFIG line" },
                    )])
                    .headed("Custom mapping"),
                ],
            },
            Section {
                id: "updates",
                title: "Updates",
                groups: vec![Group::new(vec![
                    choice("channel", "Release channel", CHANNEL_OPTIONS, 0),
                    toggle("auto_update", "Check for updates automatically", true),
                    Row::new("check_update", "Check for updates now", RowKind::Action { action: RowAction::CheckUpdate, danger: false }),
                ])],
            },
            Section {
                id: "files",
                title: "Installed files",
                groups: vec![
                    Group::new(vec![
                        Row::new("version", "Version", RowKind::Info { value: game.version.to_string() }),
                        Row::new("source", "Source", RowKind::Info { value: game.source.host().to_string() }),
                        Row::new("open_folder", "Open install folder", RowKind::Action { action: RowAction::OpenFolder, danger: false }),
                        Row::new("verify", "Verify files", RowKind::Action { action: RowAction::Verify, danger: false }),
                    ]),
                    Group::new(vec![Row::new("uninstall", "Uninstall", RowKind::Action { action: RowAction::Uninstall, danger: true })])
                        .headed("Remove")
                        .noted("Removes the app from this device. Your own game file is never touched."),
                ],
            },
        ],
    }
}

/// Reclaw's own settings (Main menu, Settings). `launch` is every launch setting the display can
/// honor (`all_specs`): the defaults for games, applied to the ones that support each.
pub fn global_settings(launch: &[SettingSpec]) -> Schema {
    let mut schema = global_sections();
    if let Some(section) = launch_section("games", "Game defaults", launch) {
        // After Controller, before Library.
        let at = schema.sections.iter().position(|s| s.id == "library").unwrap_or(schema.sections.len());
        schema.sections.insert(at, section);
    }
    schema
}

fn global_sections() -> Schema {
    Schema {
        title: "Settings".into(),
        sections: vec![
            Section {
                id: "interface",
                title: "Interface",
                groups: vec![Group::new(vec![
                    choice(KEY_INTERFACE_MODE, "Interface", MODE_OPTIONS, 0)
                        .described("Auto picks Deck mode under a console session; the others force one."),
                    choice(KEY_THEME, "Theme", THEME_OPTIONS, 0),
                    choice("ui_scale", "UI scale", SCALE_OPTIONS, 0),
                    choice("safe_zone", "Screen edge margin", SAFE_ZONE_OPTIONS, 1)
                        .described("Raise it if the edges of your TV are cut off."),
                ])],
            },
            Section {
                id: "motion",
                title: "Motion",
                groups: vec![
                    Group::new(vec![toggle(KEY_REDUCE_MOTION, "Reduce motion", false).described("Cut straight between pages everywhere.")]),
                    Group::new(vec![
                        choice(KEY_DESKTOP_INTENSITY, "Page transitions", INTENSITY_OPTIONS, 1),
                        choice(KEY_DESKTOP_STYLE, "Default style", STYLE_OPTIONS, 0)
                            .described("How a page arrives unless it picks its own."),
                        toggle(KEY_DESKTOP_PAGE_STYLES, "Pages pick their own style", true).described("A game page brings its banner in."),
                    ])
                    .headed("Desktop"),
                    Group::new(vec![
                        choice(KEY_CONSOLE_INTENSITY, "Page transitions", INTENSITY_OPTIONS, 2),
                        choice(KEY_CONSOLE_STYLE, "Default style", STYLE_OPTIONS, 0)
                            .described("How a page arrives unless it picks its own."),
                        toggle(KEY_CONSOLE_PAGE_STYLES, "Pages pick their own style", true).described("A game page brings its banner in."),
                    ])
                    .headed("Deck mode"),
                ],
            },
            Section {
                id: "controller",
                title: "Controller",
                groups: vec![Group::new(vec![
                    choice("confirm_button", "Confirm button", CONFIRM_OPTIONS, 0),
                    toggle("rumble", "Vibration", true),
                ])],
            },
            Section {
                id: "library",
                title: "Library",
                groups: vec![
                    Group::new(vec![toggle("check_on_launch", "Check for updates when Reclaw starts", true)]),
                    Group::new(vec![Row::new(
                        "default_location",
                        "Default install location",
                        RowKind::Text { field: TextField::DefaultLocation, placeholder: "~/Reclaw/Apps" },
                    )])
                    .headed("Install location"),
                ],
            },
            Section {
                id: "about",
                title: "About",
                groups: vec![Group::new(vec![
                    Row::new("version", "Reclaw", RowKind::Info { value: env!("CARGO_PKG_VERSION").to_string() }),
                    Row::new("toolkit", "Interface toolkit", RowKind::Info { value: "Freya 0.5".to_string() }),
                ])],
            },
        ],
    }
}
