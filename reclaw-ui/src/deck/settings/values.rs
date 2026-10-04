use std::collections::HashMap;

use super::schema::{RowKind, Schema};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingValue {
    Bool(bool),
    Choice(usize),
}

/// Where a setting lives: Reclaw-wide, or one app's.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SettingsTarget {
    Global,
    App(u32),
}

impl SettingsTarget {
    pub fn app(self) -> Option<u32> {
        match self {
            Self::Global => None,
            Self::App(id) => Some(id),
        }
    }
}

/// A change to report to the host, which persists it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SettingChange {
    pub app: Option<u32>,
    pub key: &'static str,
    pub value: SettingValue,
}

/// Edited values. A key that was never touched reads its schema default, so persisting is the
/// host's job and an empty store is a valid starting point.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct SettingsValues {
    map: HashMap<(SettingsTarget, &'static str), SettingValue>,
}

impl SettingsValues {
    pub fn set(&mut self, target: SettingsTarget, key: &'static str, value: SettingValue) {
        self.map.insert((target, key), value);
    }

    /// Seed from what the host persisted.
    pub fn load(&mut self, changes: impl IntoIterator<Item = SettingChange>) {
        for c in changes {
            let target = c.app.map_or(SettingsTarget::Global, SettingsTarget::App);
            self.set(target, c.key, c.value);
        }
    }

    pub fn get(&self, target: SettingsTarget, key: &'static str) -> Option<SettingValue> {
        self.map.get(&(target, key)).copied()
    }

    pub fn toggle(&self, target: SettingsTarget, key: &'static str, default: bool) -> bool {
        match self.get(target, key) {
            Some(SettingValue::Bool(b)) => b,
            _ => default,
        }
    }

    pub fn choice(&self, target: SettingsTarget, key: &'static str, default: usize) -> usize {
        match self.get(target, key) {
            Some(SettingValue::Choice(i)) => i,
            _ => default,
        }
    }

    /// The text to show for a row's current value.
    pub fn display(&self, target: SettingsTarget, schema: &Schema, section: usize, row: usize) -> Option<String> {
        let row = schema.row(section, row)?;
        Some(match &row.kind {
            RowKind::Choice { options, default } => options.get(self.choice(target, row.key, *default)).copied().unwrap_or("").to_string(),
            RowKind::Info { value } => value.clone(),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untouched_keys_read_their_defaults() {
        let v = SettingsValues::default();
        assert!(v.toggle(SettingsTarget::Global, "rumble", true));
        assert_eq!(v.choice(SettingsTarget::Global, "ui_scale", 2), 2);
    }

    #[test]
    fn app_and_global_values_do_not_collide() {
        let mut v = SettingsValues::default();
        v.set(SettingsTarget::Global, "x", SettingValue::Bool(true));
        v.set(SettingsTarget::App(1), "x", SettingValue::Bool(false));
        assert!(v.toggle(SettingsTarget::Global, "x", false));
        assert!(!v.toggle(SettingsTarget::App(1), "x", true));
        assert!(v.toggle(SettingsTarget::App(2), "x", true), "another app still reads the default");
    }

    #[test]
    fn load_applies_persisted_changes() {
        let mut v = SettingsValues::default();
        v.load([SettingChange { app: Some(3), key: "swap_ab", value: SettingValue::Bool(true) }]);
        assert!(v.toggle(SettingsTarget::App(3), "swap_ab", false));
    }
}
