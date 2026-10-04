use std::collections::{BTreeMap, BTreeSet};

use reclaw_games::settings::SettingsLayer;
use serde::{Deserialize, Serialize};

/// A stored setting. Written as a plain TOML bool, integer or string, so the file reads naturally.
/// A choice is stored by its label, not its position, so reordering the options in a later release
/// does not change what people picked.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrefValue {
    Bool(bool),
    Int(i64),
    Text(String),
}

impl PrefValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }
}

/// The launch settings the user chose: defaults for every game, and per-game overrides.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchPrefs {
    pub defaults: SettingsLayer,
    pub apps: BTreeMap<u32, SettingsLayer>,
}

/// Where the window was. Applied at startup if the monitor is still connected.
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowPrefs {
    /// Logical size of the desktop window.
    pub size: Option<(f32, f32)>,
    /// Logical position on the virtual desktop. Wayland compositors ignore it.
    pub position: Option<(i32, i32)>,
    pub maximized: bool,
    /// The monitor the desktop window was on (a connector name such as `DP-1`).
    pub monitor: Option<String>,
    /// The monitor Deck mode opens on, when not the one the window is on.
    pub deck_monitor: Option<String>,
}

/// Everything persisted. Adding a field never needs a migration: every field has a default, so an
/// older file reads fine, and an unknown key from a newer file is ignored.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Format version of the file, see `migrate`.
    pub version: u32,
    /// Reclaw's own settings rows by key (`interface_mode`, `ui_scale`, ...). The settings schema
    /// owns the meaning; this file only stores what was chosen.
    pub global: BTreeMap<String, PrefValue>,
    /// Per-game settings rows (release channel, launch options ...), by game id then key.
    pub apps: BTreeMap<u32, BTreeMap<String, PrefValue>>,
    pub launch: LaunchPrefs,
    /// Games marked with the star.
    pub favorites: BTreeSet<u32>,
    pub window: WindowPrefs,
}

impl Preferences {
    /// The format this build writes.
    pub const VERSION: u32 = 1;
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            global: BTreeMap::new(),
            apps: BTreeMap::new(),
            launch: LaunchPrefs::default(),
            favorites: BTreeSet::new(),
            window: WindowPrefs::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use reclaw_games::settings::{SettingKey, SettingValue};

    use super::*;

    fn sample() -> Preferences {
        let mut p = Preferences::default();
        p.global.insert("interface_mode".into(), PrefValue::Text("Deck".into()));
        p.global.insert("rumble".into(), PrefValue::Bool(false));
        p.global.insert("ui_scale_percent".into(), PrefValue::Int(125));
        p.apps.entry(3).or_default().insert("keep_prerelease".into(), PrefValue::Bool(true));
        p.launch.defaults.set(SettingKey::Vsync, SettingValue::Bool(false));
        p.launch.apps.entry(2).or_default().set(SettingKey::Msaa, SettingValue::choice("4x"));
        p.favorites.extend([1, 4]);
        p.window = WindowPrefs {
            size: Some((1280., 800.)),
            position: Some((10, 20)),
            maximized: true,
            monitor: Some("DP-1".into()),
            deck_monitor: None,
        };
        p
    }

    #[test]
    fn everything_round_trips_through_toml() {
        let p = sample();
        let text = toml::to_string_pretty(&p).expect("encode");
        let back: Preferences = toml::from_str(&text).expect("decode");
        assert_eq!(back, p, "{text}");
    }

    #[test]
    fn the_file_reads_naturally() {
        let text = toml::to_string_pretty(&sample()).expect("encode");
        assert!(text.contains("interface_mode = \"Deck\""), "{text}");
        assert!(text.contains("rumble = false"), "{text}");
        assert!(text.contains("favorites = [1, 4]") || text.contains("favorites = [\n"), "{text}");
    }

    #[test]
    fn missing_fields_take_defaults_and_unknown_ones_are_ignored() {
        let p: Preferences = toml::from_str("version = 1\nfavorites = [7]\nfuture_thing = true\n[future_table]\nx = 1\n").expect("decode");
        assert_eq!(p.favorites.iter().copied().collect::<Vec<_>>(), vec![7]);
        assert!(p.global.is_empty() && p.window == WindowPrefs::default());
    }

    #[test]
    fn an_empty_file_is_the_defaults() {
        let p: Preferences = toml::from_str("").expect("decode");
        assert_eq!(p, Preferences::default());
    }
}
