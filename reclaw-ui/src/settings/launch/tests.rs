use reclaw_config::LaunchPrefs;
use reclaw_games::{
    fixtures::sample_projects,
    settings::{DisplayEnvironment, SettingKey, SettingValue, Size},
};

use super::*;

fn ctx<'a>(env: &'a DisplayEnvironment, projects: &'a [reclaw_games::project::ProjectInfo], prefs: &'a LaunchPrefs) -> LaunchContext<'a> {
    LaunchContext { env, projects, prefs }
}

#[test]
fn the_defaults_page_lists_everything_the_display_can_honor() {
    let (env, prefs, projects) = (DisplayEnvironment::unknown(), LaunchPrefs::default(), sample_projects());
    let keys: Vec<SettingKey> = ctx(&env, &projects, &prefs).specs(SettingsTarget::Global).into_iter().map(|s| s.key).collect();
    assert!(keys.contains(&SettingKey::Resolution) && keys.contains(&SettingKey::Msaa) && keys.contains(&SettingKey::UpscaleMethod));
}

#[test]
fn a_game_page_lists_only_what_the_game_declares_and_nothing_for_a_game_that_declares_none() {
    let (env, prefs, projects) = (DisplayEnvironment::unknown(), LaunchPrefs::default(), sample_projects());
    let c = ctx(&env, &projects, &prefs);
    let starfall: Vec<SettingKey> = c.specs(SettingsTarget::App(1)).into_iter().map(|s| s.key).collect();
    assert!(starfall.contains(&SettingKey::WindowMode) && !starfall.contains(&SettingKey::Msaa), "{starfall:?}");
    assert!(c.specs(SettingsTarget::App(3)).is_empty(), "Kart Ruins declares nothing");
    assert!(c.specs(SettingsTarget::App(999)).is_empty(), "an unknown game offers nothing");
    assert!(c.control(SettingsTarget::App(1), SettingKey::Msaa).is_none(), "not declared, so not offered even by name");
}

#[test]
fn with_nothing_chosen_the_row_says_the_game_decides() {
    let (env, prefs, projects) = (DisplayEnvironment::unknown(), LaunchPrefs::default(), sample_projects());
    let control = ctx(&env, &projects, &prefs).control(SettingsTarget::App(1), SettingKey::Vsync).expect("offered");
    assert_eq!((control.summary.as_str(), control.selected), ("Game's own", 0));
    assert_eq!(control.options[0].value, None);
    assert_eq!(control.options.len(), 3, "game's own, on, off");
}

#[test]
fn a_default_shows_on_a_game_that_has_no_choice_of_its_own() {
    let (env, projects) = (DisplayEnvironment::unknown(), sample_projects());
    let mut prefs = LaunchPrefs::default();
    prefs.defaults.set(SettingKey::Resolution, SettingValue::Size(Size::new(1920, 1080)));
    let control = ctx(&env, &projects, &prefs).control(SettingsTarget::App(1), SettingKey::Resolution).expect("offered");
    assert_eq!(control.summary, "Default (1920x1080)");
    assert_eq!(control.options[0].label, "Default (1920x1080)");
    assert_eq!(control.selected, 0, "no override is chosen");
}

#[test]
fn a_games_own_choice_overrides_the_default_and_is_selected_in_the_picker() {
    let (env, projects) = (DisplayEnvironment::unknown(), sample_projects());
    let mut prefs = LaunchPrefs::default();
    prefs.defaults.set(SettingKey::Vsync, SettingValue::Bool(true));
    prefs.apps.entry(1).or_default().set(SettingKey::Vsync, SettingValue::Bool(false));
    let control = ctx(&env, &projects, &prefs).control(SettingsTarget::App(1), SettingKey::Vsync).expect("offered");
    assert_eq!(control.summary, "Off");
    assert_eq!(control.options[control.selected].value, Some(SettingValue::Bool(false)));
}

#[test]
fn a_default_the_game_cannot_take_does_not_apply_to_it() {
    let (env, projects) = (DisplayEnvironment::unknown(), sample_projects());
    let mut prefs = LaunchPrefs::default();
    // Skyward Quest takes 4x at most; 8x was a default set for another game.
    prefs.defaults.set(SettingKey::Msaa, SettingValue::choice("8x"));
    let control = ctx(&env, &projects, &prefs).control(SettingsTarget::App(2), SettingKey::Msaa).expect("offered");
    assert_eq!(control.summary, "Game's own", "an 8x default does not apply to a game that stops at 4x");
}

#[test]
fn the_defaults_page_says_game_own_for_the_clear_entry() {
    let (env, prefs, projects) = (DisplayEnvironment::unknown(), LaunchPrefs::default(), sample_projects());
    let control = ctx(&env, &projects, &prefs).control(SettingsTarget::Global, SettingKey::FrameLimit).expect("offered");
    assert_eq!(control.options[0].label, "Game's own");
}
