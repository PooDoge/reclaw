//! The two schemas Reclaw ships: per-app Properties and global Settings.
use super::schema::*;
use crate::model::GameEntry;

pub const MODE_OPTIONS: &[&str] = &["Auto", "Desktop", "Deck"];
pub const SCALE_OPTIONS: &[&str] = &["100%", "125%", "150%"];
pub const SAFE_ZONE_OPTIONS: &[&str] = &["Off", "Small", "Large"];
pub const CONFIRM_OPTIONS: &[&str] = &["Bottom button (A, cross)", "Right button (B, circle)"];
pub const CHANNEL_OPTIONS: &[&str] = &["Stable", "Pre-release"];

/// Keys that need more than a stored value when they change.
pub const KEY_INTERFACE_MODE: &str = "interface_mode";

fn toggle(key: &'static str, label: &'static str, default: bool) -> Row {
    Row::new(key, label, RowKind::Toggle { default })
}

fn choice(key: &'static str, label: &'static str, options: &'static [&'static str], default: usize) -> Row {
    Row::new(key, label, RowKind::Choice { options, default })
}

/// Properties of one installed or installable app (the "Properties..." page).
pub fn app_properties(game: &GameEntry) -> Schema {
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

/// Reclaw's own settings (Main menu, Settings).
pub fn global_settings() -> Schema {
    Schema {
        title: "Settings".into(),
        sections: vec![
            Section {
                id: "interface",
                title: "Interface",
                groups: vec![Group::new(vec![
                    choice(KEY_INTERFACE_MODE, "Interface", MODE_OPTIONS, 0)
                        .described("Auto picks Deck mode under a console session; the others force one."),
                    choice("ui_scale", "UI scale", SCALE_OPTIONS, 0),
                    choice("safe_zone", "Screen edge margin", SAFE_ZONE_OPTIONS, 1)
                        .described("Raise it if the edges of your TV are cut off."),
                ])],
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
