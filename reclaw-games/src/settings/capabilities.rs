use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    key::SettingKey,
    value::{SettingValue, Size},
};

/// What a game declares it can honor. Written by catalog maintainers, one entry per project.
/// An empty list means the game page shows no launch settings at all.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Capabilities {
    pub settings: Vec<Binding>,
}

impl Capabilities {
    pub fn binding(&self, key: SettingKey) -> Option<&Binding> {
        self.settings.iter().find(|b| b.key == key)
    }

    pub fn supports(&self, key: SettingKey) -> bool {
        self.binding(key).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.settings.is_empty()
    }
}

/// One supported setting: which values are allowed and how a value reaches the game.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Binding {
    pub key: SettingKey,
    #[serde(default)]
    pub constraint: Constraint,
    /// Every output runs for a chosen value (a resolution writes a width and a height).
    pub outputs: Vec<Output>,
}

/// Narrows what the launcher offers for this game below the key's standard range.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Constraint {
    #[default]
    None,
    /// Only these choice ids.
    Choices(Vec<String>),
    Range {
        min: i64,
        max: i64,
    },
    Sizes(Vec<Size>),
}

/// How one chosen value is applied.
///
/// `when` limits the output to one value ("only add `--fullscreen` when the mode is exclusive").
/// Templates may use `{value}` (the value as text), `{width}`, `{height}` and `{mapped}`;
/// `map` rewrites a choice id or a bool (`"true"`/`"false"`) to the game's own word before it
/// is substituted as `{mapped}`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Output {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<SettingValue>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub map: BTreeMap<String, String>,
    pub target: Target,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Target {
    /// Arguments appended to the command line, each a template: `["--width", "{width}"]`.
    Args {
        args: Vec<String>,
    },
    Env {
        name: String,
        value: String,
    },
    /// Set `path` (dotted: `Graphics.Resolution.Width`; INI: `section.key`) in a config file the
    /// game reads at start.
    Config {
        file: ConfigPath,
        format: ConfigFormat,
        path: String,
        value: String,
        #[serde(default)]
        value_type: ValueType,
    },
}

/// What the rendered template becomes in a typed config file.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    /// `true`/`false` become booleans, whole numbers become integers, anything else text.
    #[default]
    Auto,
    String,
    Int,
    Bool,
    Float,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFormat {
    Json,
    Toml,
    /// `[section]` headers and `key = value` lines.
    Ini,
    /// One `key value` or `key=value` per line, no sections (many N64 and PC ports).
    KeyValue,
}

/// Where a game keeps a file, relative to a base the host resolves per install.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfigPath {
    pub base: Base,
    pub relative: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Base {
    /// The game's install directory.
    Install,
    /// The game's own config directory (`~/.config/<game>`, `%APPDATA%\<game>`).
    Config,
    /// The game's data directory.
    Data,
}
