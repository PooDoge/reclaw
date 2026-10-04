use reclaw_config::WindowPrefs;
use reclaw_games::settings::{SettingKey, SettingValue as LaunchValue};

use super::state;
use crate::{
    settings::{SettingChange, SettingValue, SettingsTarget},
    store::{AppAction, AppState},
};

#[test]
fn what_is_saved_comes_back_and_what_is_not_does_not() {
    let mut s = state();
    s.reduce(AppAction::ToggleFavorite(4));
    s.reduce(AppAction::Setting(SettingChange { app: None, key: "rumble", value: SettingValue::Bool(false) }));
    s.reduce(AppAction::LaunchSetting { app: Some(2), key: SettingKey::Msaa, value: Some(LaunchValue::choice("4x")) });
    s.reduce(AppAction::Window(WindowPrefs { size: Some((1200., 800.)), maximized: true, ..WindowPrefs::default() }));
    s.reduce(AppAction::SetKeyboardInset(300.));

    let back = AppState::from_prefs(s.to_prefs());
    assert!(back.favorites.contains(&4));
    assert!(!back.settings.toggle(SettingsTarget::Global, "rumble", true));
    assert_eq!(back.launch.apps[&2].get(SettingKey::Msaa), Some(&LaunchValue::choice("4x")));
    assert!(back.window.maximized);
    assert_eq!(back.keyboard_inset, 0., "device facts are not saved");
    assert!(back.games.is_empty(), "the library is the host's, not the settings file's");
}
