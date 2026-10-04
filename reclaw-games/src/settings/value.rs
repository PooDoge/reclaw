use serde::{Deserialize, Serialize};

/// A width and height in pixels.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// `1920x1080`, the form shown to people and written by most games.
    pub fn label(self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

/// A value a user can choose for a setting. Which variant is valid depends on the key's
/// [`ValueKind`]; a mismatch is treated as "not set" (see `adapt`).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    /// The `id` of one of the kind's [`Choice`]s.
    Choice(String),
    Size(Size),
    /// "Whatever the monitor is". Only meaningful for a resolution.
    Native,
}

impl SettingValue {
    pub fn choice(id: &str) -> Self {
        Self::Choice(id.to_string())
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
}

impl Choice {
    pub fn new(id: &str, label: &str) -> Self {
        Self { id: id.to_string(), label: label.to_string() }
    }
}

/// The shape of a setting's values for one game on one display: what the settings page offers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ValueKind {
    Bool,
    /// Whole numbers from `min` to `max` in steps of `step`. `zero_label` names 0 when 0 means
    /// "off" ("No limit" for a frame cap).
    Int {
        min: i64,
        max: i64,
        step: i64,
        unit: Option<&'static str>,
        zero_label: Option<&'static str>,
    },
    Choice(Vec<Choice>),
    /// One of `options`, or the monitor's native size when `native` is set.
    Size {
        options: Vec<Size>,
        native: bool,
    },
}
