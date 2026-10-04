//! How settings values are written to the settings file and read back.
//!
//! The file stores a toggle as a bool, a choice as the *label* of the option (so reordering options
//! in a later release does not change what people picked) and a text as a string. Reading looks each
//! stored key up in the shipped schemas: a key no row has (a setting removed since, or from a newer
//! Reclaw) or a label no option has (renamed) is left out, and that row shows its default.
use std::collections::BTreeMap;

use reclaw_config::PrefValue;

use super::{
    builders::{app_properties, global_settings},
    schema::{Row, RowKind, Schema},
    values::{SettingValue, SettingsTarget, SettingsValues},
};
use crate::model::{AppStatus, GameEntry, RunState, Source};

type Table = BTreeMap<String, PrefValue>;

/// The rows of both schemas, to find the static key and the options of a stored name.
fn rows(target: SettingsTarget) -> Vec<Row> {
    let schema: Schema = match target {
        SettingsTarget::Global => global_settings(&[]),
        SettingsTarget::App(_) => {
            // The rows do not depend on the game except for read-only info lines.
            let placeholder = GameEntry {
                id: 0,
                title: "".into(),
                project: "".into(),
                version: "".into(),
                source: Source::Manual,
                status: AppStatus::Installed,
                tags: Vec::new(),
                run: RunState::Idle,
            };
            app_properties(&placeholder, &[])
        }
    };
    schema.sections.into_iter().flat_map(|s| s.groups).flat_map(|g| g.rows).collect()
}

/// Values to file form: global rows, and per-app rows by app id.
pub fn to_prefs(values: &SettingsValues) -> (Table, BTreeMap<u32, Table>) {
    let (global_rows, app_rows) = (rows(SettingsTarget::Global), rows(SettingsTarget::App(0)));
    let mut global = Table::new();
    let mut apps: BTreeMap<u32, Table> = BTreeMap::new();
    let mut put = |target: SettingsTarget, key: &str, value: PrefValue| match target {
        SettingsTarget::Global => {
            global.insert(key.to_string(), value);
        }
        SettingsTarget::App(id) => {
            apps.entry(id).or_default().insert(key.to_string(), value);
        }
    };
    for (target, key, value) in values.entries() {
        let table_rows = if target == SettingsTarget::Global { &global_rows } else { &app_rows };
        let Some(row) = table_rows.iter().find(|r| r.key == key) else { continue };
        match (value, &row.kind) {
            (SettingValue::Bool(b), RowKind::Toggle { .. }) => put(target, key, PrefValue::Bool(b)),
            (SettingValue::Choice(i), RowKind::Choice { options, .. }) => {
                if let Some(label) = options.get(i) {
                    put(target, key, PrefValue::Text((*label).to_string()));
                }
            }
            _ => {}
        }
    }
    for (target, key, text) in values.text_entries() {
        put(target, key, PrefValue::Text(text.to_string()));
    }
    (global, apps)
}

/// File form back to values. Anything the shipped schemas do not recognise is dropped.
pub fn from_prefs(global: &Table, apps: &BTreeMap<u32, Table>) -> SettingsValues {
    let mut values = SettingsValues::default();
    let mut read = |target: SettingsTarget, table: &Table, rows: &[Row]| {
        for row in rows {
            let Some(stored) = table.get(row.key) else { continue };
            match (&row.kind, stored) {
                (RowKind::Toggle { .. }, PrefValue::Bool(b)) => values.set(target, row.key, SettingValue::Bool(*b)),
                (RowKind::Choice { options, .. }, PrefValue::Text(label)) => {
                    if let Some(i) = options.iter().position(|o| o == label) {
                        values.set(target, row.key, SettingValue::Choice(i));
                    }
                }
                (RowKind::Text { .. }, PrefValue::Text(text)) => values.set_text(target, row.key, text.clone()),
                _ => {}
            }
        }
    };
    read(SettingsTarget::Global, global, &rows(SettingsTarget::Global));
    let app_rows = rows(SettingsTarget::App(0));
    for (id, table) in apps {
        read(SettingsTarget::App(*id), table, &app_rows);
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every toggle and choice row of both schemas, set to a non-default value.
    fn everything_set() -> SettingsValues {
        let mut v = SettingsValues::default();
        for (target, rows) in
            [(SettingsTarget::Global, rows(SettingsTarget::Global)), (SettingsTarget::App(7), rows(SettingsTarget::App(7)))]
        {
            for row in rows {
                match &row.kind {
                    RowKind::Toggle { default } => v.set(target, row.key, SettingValue::Bool(!default)),
                    RowKind::Choice { options, default } => v.set(target, row.key, SettingValue::Choice((default + 1) % options.len())),
                    RowKind::Text { .. } => v.set_text(target, row.key, format!("text for {}", row.key)),
                    _ => {}
                }
            }
        }
        v
    }

    #[test]
    fn every_shipped_toggle_choice_and_text_round_trips() {
        let values = everything_set();
        let (global, apps) = to_prefs(&values);
        let back = from_prefs(&global, &apps);
        let sorted = |v: &SettingsValues| {
            let mut e: Vec<_> = v.entries().map(|(t, k, v)| (format!("{t:?}"), k, v)).collect();
            e.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
            let mut t: Vec<_> = v.text_entries().map(|(t, k, s)| (format!("{t:?}"), k, s.to_string())).collect();
            t.sort();
            (e, t)
        };
        assert_eq!(sorted(&back), sorted(&values));
        assert!(!global.is_empty() && apps.contains_key(&7));
    }

    #[test]
    fn a_choice_is_stored_by_its_label() {
        let mut v = SettingsValues::default();
        v.set(SettingsTarget::Global, "interface_mode", SettingValue::Choice(2));
        let (global, _) = to_prefs(&v);
        assert_eq!(global.get("interface_mode"), Some(&PrefValue::Text("Deck".into())));
    }

    #[test]
    fn unknown_keys_renamed_labels_and_wrong_types_are_left_out() {
        let mut global = Table::new();
        global.insert("setting_from_the_future".into(), PrefValue::Bool(true));
        global.insert("interface_mode".into(), PrefValue::Text("Console".into()));
        global.insert("rumble".into(), PrefValue::Text("yes".into()));
        let values = from_prefs(&global, &BTreeMap::new());
        assert_eq!(values.entries().count(), 0);
    }

    #[test]
    fn values_for_a_game_that_is_not_in_the_library_are_kept() {
        let mut v = SettingsValues::default();
        v.set(SettingsTarget::App(404), "keep_prerelease", SettingValue::Bool(true));
        let (_, apps) = to_prefs(&v);
        let back = from_prefs(&Table::new(), &apps);
        assert!(back.toggle(SettingsTarget::App(404), "keep_prerelease", false));
    }

    #[test]
    fn install_location_is_a_form_field_not_a_setting() {
        assert_eq!(crate::settings::TextField::InstallLocation.key(), None);
        assert_eq!(crate::settings::TextField::LaunchOptions.key(), Some("launch_options"));
    }
}
