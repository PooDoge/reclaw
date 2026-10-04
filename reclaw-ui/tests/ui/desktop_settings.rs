//! The desktop settings pages: Reclaw's settings and a game's properties, from the schema Deck mode
//! shares. A change goes through the store and is reported to the host as the same effect.
use reclaw_games::settings::{SettingKey, SettingValue as Launch, Size};
use reclaw_ui::{
    effect::Effect,
    nav::Route,
    settings::{SettingChange, SettingValue, SettingsTarget},
    shell::{DevOverrides, MotionOverride},
};

use crate::common::*;

fn calm() -> DevOverrides {
    DevOverrides { motion: Some(MotionOverride::Reduced), ..DevOverrides::default() }
}

fn settings() -> Session {
    Mount::desktop().dev(calm()).start_at(Route::Settings {})
}

#[test]
fn the_settings_list_their_sections_and_show_the_first() {
    let mut s = settings();
    for section in ["Interface", "Motion", "Controller", "Game defaults", "Library", "About"] {
        assert!(s.has_label(section), "{section}: {:?}", s.labels());
    }
    assert!(s.has_label("Theme") && s.has_label("UI scale"), "the Interface rows: {:?}", s.labels());
    s.snapshot("desktop-settings");
}

#[test]
fn a_toggle_row_changes_the_store_and_tells_the_host() {
    let mut s = settings();
    s.click_label("Controller");
    s.click_label("Vibration");
    assert_eq!(s.take_effects(), vec![Effect::Setting(SettingChange { app: None, key: "rumble", value: SettingValue::Bool(false) })]);
    assert!(!s.store().snapshot().settings.toggle(SettingsTarget::Global, "rumble", true));
}

#[test]
fn a_choice_row_opens_a_picker_and_choosing_saves_the_label_choice() {
    let mut s = settings();
    s.click_label("Theme");
    assert!(s.has_label("Midnight") && s.has_label("Daylight"), "the picker lists both: {:?}", s.labels());
    s.click_label("Daylight");
    assert_eq!(s.store().snapshot().settings.choice(SettingsTarget::Global, "theme", 0), 1);
    assert!(s.take_effects().contains(&Effect::Setting(SettingChange { app: None, key: "theme", value: SettingValue::Choice(1) })));
}

#[test]
fn the_interface_row_switches_the_interface_too() {
    let mut s = settings();
    // The section button and the row share the name; the topmost is the row. Its picker lists
    // Auto, Desktop and Deck.
    s.click_label("Interface");
    s.click_label("Desktop");
    let effects = s.take_effects();
    assert!(effects.iter().any(|e| matches!(e, Effect::SetMode(_))), "{effects:?}");
}

#[test]
fn the_motion_section_changes_the_transitions() {
    let mut s = settings();
    s.click_label("Motion");
    assert!(s.has_label("Reduce motion") && s.has_label("DESKTOP") && s.has_label("DECK MODE"), "{:?}", s.labels());
    s.click_label("Reduce motion");
    assert!(s.store().snapshot().settings.toggle(SettingsTarget::Global, "motion_reduce", false));
}

#[test]
fn game_defaults_list_the_launch_settings_and_a_choice_is_stored_as_the_default() {
    let mut s = settings();
    s.click_label("Game defaults");
    for row in ["Window mode", "Resolution", "Aspect ratio", "Vertical sync", "Frame rate limit", "Upscaling", "Anti-aliasing"] {
        assert!(s.has_label(row), "{row}: {:?}", s.labels());
    }
    s.click_label("Resolution");
    assert!(s.has_label("Game's own") && s.has_label("Native") && s.has_label("1920x1080"), "{:?}", s.labels());
    s.click_label("1920x1080");
    let state = s.store().snapshot();
    assert_eq!(state.launch.defaults.get(SettingKey::Resolution), Some(&Launch::Size(Size::new(1920, 1080))));
    assert_eq!(
        s.take_effects(),
        vec![Effect::LaunchSetting { app: None, key: SettingKey::Resolution, value: Some(Launch::Size(Size::new(1920, 1080))) }]
    );
}

#[test]
fn a_game_that_supports_launch_settings_shows_only_those_and_inherits_the_default() {
    let mut s = Mount::desktop().dev(calm()).start_at(Route::GameSettings { id: 1 });
    assert!(s.has_label("Display and graphics"), "{:?}", s.labels());
    s.click_label("Display and graphics");
    for row in ["Window mode", "Resolution", "Vertical sync", "Frame rate limit"] {
        assert!(s.has_label(row), "{row}: {:?}", s.labels());
    }
    assert!(!s.has_label("Anti-aliasing"), "Starfall 64 does not declare it: {:?}", s.labels());
    assert!(s.has_label("Game's own"), "nothing chosen yet");

    // A default set on the defaults page shows on the game, and choosing "Off" overrides it for this game only.
    let store = s.store();
    s.runner.run_in(|| {
        store.dispatch(reclaw_ui::store::AppAction::LaunchSetting { app: None, key: SettingKey::Vsync, value: Some(Launch::Bool(true)) })
    });
    s.settle();
    assert!(s.has_label("Default (On)"), "{:?}", s.labels());
    s.click_label("Vertical sync");
    s.click_label("Off");
    assert_eq!(store.snapshot().launch.apps[&1].get(SettingKey::Vsync), Some(&Launch::Bool(false)));
    assert_eq!(store.snapshot().launch.defaults.get(SettingKey::Vsync), Some(&Launch::Bool(true)), "the default is untouched");
}

#[test]
fn a_game_with_no_launch_settings_has_no_such_section() {
    let mut s = Mount::desktop().dev(calm()).start_at(Route::GameSettings { id: 3 });
    assert!(s.has_label("General") && s.has_label("Installed files"), "{:?}", s.labels());
    assert!(!s.has_label("Display and graphics"), "nothing to show for Kart Ruins: {:?}", s.labels());
    s.snapshot("desktop-game-settings-none");
}

#[test]
fn on_a_phone_the_sections_are_a_list_and_each_opens_as_its_own_page() {
    let mut s = Mount::desktop().size(390., 780.).dev(calm()).start_at(Route::Settings {});
    assert!(s.has_label("Interface") && s.has_label("Motion"), "{:?}", s.labels());
    assert!(!s.has_label("Theme"), "the rows are on the section's page, not the list: {:?}", s.labels());
    s.click_label("Interface");
    assert!(s.has_label("Theme"), "{:?}", s.labels());
    s.click_label("Back");
    assert!(s.has_label("Motion") && !s.has_label("Theme"), "back to the list: {:?}", s.labels());
}

#[test]
fn text_settings_are_stored_as_they_are_typed() {
    let mut s = Mount::desktop().dev(calm()).start_at(Route::GameSettings { id: 1 });
    assert!(s.has_label("Launch options"), "{:?}", s.labels());
    // Click into the field and type; every change reaches the store and the host.
    // The input sits under the row's description.
    let (left, _, _, bottom) = s.label_box("For advanced users: extra command-line arguments.").expect("the description");
    s.runner.click_cursor((f64::from(left + 80.), f64::from(bottom + 24.)));
    s.settle();
    s.type_text("--debug");
    let state = s.store().snapshot();
    assert_eq!(state.settings.text(SettingsTarget::App(1), "launch_options"), Some("--debug"));
    assert!(s.take_effects().iter().any(|e| matches!(e, Effect::TextCommitted { app: Some(1), .. })));
}
