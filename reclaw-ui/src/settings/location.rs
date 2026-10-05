//! Where new installs go. One rule for both interfaces and the host: the Library setting "Default
//! install location" when it holds something, otherwise [`FALLBACK_LOCATION`]. The form starts from
//! this each time it opens, and an empty box at submit time means the same thing.
use super::values::{SettingsTarget, SettingsValues};

/// The folder installs go in when nobody chose one.
pub const FALLBACK_LOCATION: &str = "~/Reclaw/Apps";
/// The settings key of the Library row.
pub const KEY_DEFAULT_LOCATION: &str = "default_location";

/// The folder a new install starts with in its form.
pub fn default_install_location(values: &SettingsValues) -> String {
    let stored = values.text(SettingsTarget::Global, KEY_DEFAULT_LOCATION).map(str::trim).filter(|text| !text.is_empty());
    stored.unwrap_or(FALLBACK_LOCATION).to_string()
}

/// What the form's text means: what was typed, or the default when it was left empty.
pub fn resolve_install_location(typed: &str, values: &SettingsValues) -> String {
    let typed = typed.trim();
    if typed.is_empty() { default_install_location(values) } else { typed.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_default(text: &str) -> SettingsValues {
        let mut values = SettingsValues::default();
        values.set_text(SettingsTarget::Global, KEY_DEFAULT_LOCATION, text.to_string());
        values
    }

    #[test]
    fn nothing_stored_falls_back() {
        assert_eq!(default_install_location(&SettingsValues::default()), FALLBACK_LOCATION);
    }

    #[test]
    fn the_setting_wins_and_is_trimmed() {
        assert_eq!(default_install_location(&with_default("  /mnt/games/Reclaw ")), "/mnt/games/Reclaw");
    }

    #[test]
    fn a_blank_setting_is_no_setting() {
        assert_eq!(default_install_location(&with_default("   ")), FALLBACK_LOCATION);
    }

    #[test]
    fn a_per_app_text_with_the_same_key_is_not_the_default() {
        let mut values = SettingsValues::default();
        values.set_text(SettingsTarget::App(4), KEY_DEFAULT_LOCATION, "/elsewhere".to_string());
        assert_eq!(default_install_location(&values), FALLBACK_LOCATION);
    }

    #[test]
    fn an_empty_form_uses_the_default_and_typed_text_wins() {
        let values = with_default("/data/apps");
        assert_eq!(resolve_install_location("", &values), "/data/apps");
        assert_eq!(resolve_install_location("  ", &values), "/data/apps");
        assert_eq!(resolve_install_location(" /tmp/x ", &values), "/tmp/x");
    }
}
