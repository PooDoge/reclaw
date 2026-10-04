//! What a launch-setting row shows and offers, worked out from the display, the game's declared
//! capabilities and the user's saved choices. Both interfaces use it, so the desktop picker and the
//! Deck menu list the same options and a choice means the same thing in each.
//!
//! The rules: the defaults page lists every setting the display can honor; a game's page lists only
//! the ones the game declares (and shows none of them if it declares none). A choice at game level
//! overrides the default; choosing "Default" removes the override.
use reclaw_config::LaunchPrefs;
use reclaw_games::{
    project::ProjectInfo,
    settings::{DisplayEnvironment, SettingKey, SettingSpec, SettingValue, adapt, all_specs, label, options, standard_kind, supported},
};

use super::values::SettingsTarget;

#[cfg(test)]
mod tests;

/// Everything needed to answer questions about launch rows.
#[derive(Clone, Copy)]
pub struct LaunchContext<'a> {
    pub env: &'a DisplayEnvironment,
    pub projects: &'a [ProjectInfo],
    pub prefs: &'a LaunchPrefs,
}

/// One entry of a launch row's picker.
#[derive(Clone, PartialEq, Debug)]
pub struct LaunchOption {
    /// What choosing it stores; `None` removes the choice made at this level.
    pub value: Option<SettingValue>,
    pub label: String,
}

/// A launch row as it should be drawn and what its picker offers.
#[derive(Clone, PartialEq, Debug)]
pub struct LaunchControl {
    pub spec: SettingSpec,
    /// The text on the row: the value, "Default (1920x1080)" or "Game's own".
    pub summary: String,
    pub options: Vec<LaunchOption>,
    /// Index into `options` of what is chosen now.
    pub selected: usize,
}

impl<'a> LaunchContext<'a> {
    /// The settings to list for a target: all the display can honor for the defaults, only what the
    /// game declares for a game. Empty when the game declares nothing or is unknown.
    pub fn specs(&self, target: SettingsTarget) -> Vec<SettingSpec> {
        match target {
            SettingsTarget::Global => all_specs(self.env),
            SettingsTarget::App(id) => {
                self.projects.iter().find(|p| p.id == id).map(|p| supported(&p.capabilities, self.env)).unwrap_or_default()
            }
        }
    }

    fn spec(&self, target: SettingsTarget, key: SettingKey) -> Option<SettingSpec> {
        match target {
            SettingsTarget::Global => Some(SettingSpec { key, kind: standard_kind(key, self.env)? }),
            SettingsTarget::App(_) => self.specs(target).into_iter().find(|s| s.key == key),
        }
    }

    /// The value stored at this level, if it still fits what the game or display offers.
    fn chosen(&self, target: SettingsTarget, spec: &SettingSpec) -> Option<SettingValue> {
        let layer = match target {
            SettingsTarget::Global => Some(&self.prefs.defaults),
            SettingsTarget::App(id) => self.prefs.apps.get(&id),
        };
        layer.and_then(|l| l.get(spec.key)).and_then(|v| adapt(spec, v))
    }

    /// The row for `key` on `target`, or `None` if it is not offered there.
    pub fn control(&self, target: SettingsTarget, key: SettingKey) -> Option<LaunchControl> {
        let spec = self.spec(target, key)?;
        let chosen = self.chosen(target, &spec);
        // What applies when nothing is chosen here: for a game, the default; for the defaults, the game's own.
        let inherited = match target {
            SettingsTarget::Global => None,
            SettingsTarget::App(_) => self.prefs.defaults.get(key).and_then(|v| adapt(&spec, v)),
        };
        let clear_label = match (&inherited, target) {
            (Some(v), SettingsTarget::App(_)) => format!("Default ({})", label(&spec.kind, v)),
            _ => "Game's own".to_string(),
        };
        let mut entries = vec![LaunchOption { value: None, label: clear_label.clone() }];
        entries.extend(options(&spec.kind).into_iter().map(|(value, label)| LaunchOption { value: Some(value), label }));
        let selected = entries.iter().position(|e| e.value == chosen).unwrap_or(0);
        // A stored value between the listed steps is not in `options`; it still reads right.
        let summary = match &chosen {
            Some(value) => label(&spec.kind, value),
            None => clear_label,
        };
        Some(LaunchControl { spec, summary, options: entries, selected })
    }
}
