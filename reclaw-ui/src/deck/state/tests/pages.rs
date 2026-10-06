//! The form-like pages: Install, and the Settings/Properties pages with text entry.
use super::support::*;
use crate::deck::settings::{RowKind, SettingChange, SettingValue, SettingsTarget, TextField};

fn open_install(f: &Fixture) -> DeckState {
    let mut s = f.state();
    s.click(tile(1, 3), &f.view()); // Kart Ruins: not installed
    s.apply(Confirm, &f.view());
    s
}

#[test]
fn install_is_a_page_not_a_popup() {
    let f = Fixture::new();
    let s = open_install(&f);
    assert_eq!(s.screen(), Screen::Install(3));
    assert_eq!(s.focus(), ids::INSTALL_LOCATION);
}

#[test]
fn install_submits_straight_away_with_nothing_but_the_defaults() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    // The footer buttons sit under the fields; Down from the last field lands on Install (nearest
    // by center), Left from there on Cancel.
    press(&mut s, &f, &[go(Down), go(Down)]);
    assert_eq!(s.focus(), ids::INSTALL_SUBMIT);
    assert_eq!(press(&mut s, &f, &[Confirm]), vec![Effect::SubmitInstall(3)]);
    assert_eq!(s.screen(), Screen::Game(3), "back on the game page");
}

#[test]
fn cancel_is_left_of_install() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    press(&mut s, &f, &[go(Down), go(Down), go(Left)]);
    assert_eq!(s.focus(), ids::INSTALL_CANCEL);
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.screen(), Screen::Game(3));
}

#[test]
fn the_prerelease_switch_toggles_in_place() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    assert!(!s.install_draft().prerelease);
    press(&mut s, &f, &[go(Down), Confirm]);
    assert!(s.install_draft().prerelease);
    press(&mut s, &f, &[Confirm]);
    assert!(!s.install_draft().prerelease);
}

#[test]
fn cancel_discards_the_draft() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    press(&mut s, &f, &[go(Down), Confirm, Back]);
    assert!(s.install_draft().prerelease, "the switch was on when the page was left");
    s.apply(Confirm, &f.view());
    assert_eq!(s.screen(), Screen::Install(3));
    assert!(!s.install_draft().prerelease, "a fresh draft each time");
}

#[test]
fn the_location_box_starts_from_the_library_default_each_time_the_page_opens() {
    let f = Fixture::new();
    let mut s = f.state();
    s.values_mut().set_text(SettingsTarget::Global, "default_location", " /mnt/games/Reclaw ".to_string());
    s.click(tile(1, 3), &f.view());
    s.apply(Confirm, &f.view());
    assert_eq!(s.take_text_seeds(), vec![(TextField::InstallLocation, "/mnt/games/Reclaw".to_string())]);
    assert!(s.take_text_seeds().is_empty(), "a seed is handed out once");

    // The setting changed while the page was closed: the next opening has the new one.
    press(&mut s, &f, &[Back]);
    s.values_mut().set_text(SettingsTarget::Global, "default_location", "/data/apps".to_string());
    s.apply(Confirm, &f.view());
    assert_eq!(s.take_text_seeds(), vec![(TextField::InstallLocation, "/data/apps".to_string())]);
}

#[test]
fn with_no_default_set_the_location_box_shows_the_fallback() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    assert_eq!(s.take_text_seeds(), vec![(TextField::InstallLocation, crate::settings::FALLBACK_LOCATION.to_string())]);
}

#[test]
fn text_entry_takes_the_keyboard_until_confirm_or_back() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    assert_eq!(press(&mut s, &f, &[Confirm]), vec![Effect::BeginTextEntry(TextField::InstallLocation)]);
    assert_eq!(s.text_entry(), Some(TextField::InstallLocation));
    let focus = s.focus();
    assert!(press(&mut s, &f, &[go(Down), QuickAccess, MainMenu]).is_empty(), "navigation is ignored while typing");
    assert_eq!(s.focus(), focus);
    assert_eq!(press(&mut s, &f, &[Back]), vec![Effect::EndTextEntry(TextField::InstallLocation)]);
    assert_eq!(s.text_entry(), None);
    assert_eq!(s.screen(), Screen::Install(3), "Back ended the typing, it did not leave the page");
}

#[test]
fn reveal_targets_are_fields_in_the_body_never_the_footer() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    let (top, bottom) = s.reveal_target(&f.view()).expect("a body field");
    assert_eq!((top, bottom), (0., 140.), "the tall location field");
    press(&mut s, &f, &[go(Down), go(Down)]);
    assert_eq!(s.focus(), ids::INSTALL_SUBMIT);
    assert_eq!(s.reveal_target(&f.view()), None);
}

#[test]
fn the_settings_text_boxes_start_from_what_is_stored() {
    let f = Fixture::new();
    let mut s = f.state();
    s.values_mut().set_text(SettingsTarget::Global, "default_location", "/data/apps".to_string());
    press(&mut s, &f, &[MainMenu, go(Down), go(Down), go(Down), go(Down), Confirm]);
    assert_eq!(s.screen(), Screen::Settings(SettingsTarget::Global));
    assert_eq!(s.take_text_seeds(), vec![(TextField::DefaultLocation, "/data/apps".to_string())]);
}

fn open_global_settings(f: &Fixture) -> DeckState {
    let mut s = f.state();
    // Main menu: Library, Catalog, Downloads, Mods, Settings.
    press(&mut s, f, &[MainMenu, go(Down), go(Down), go(Down), go(Down), Confirm]);
    s
}

#[test]
fn settings_open_two_pane_on_a_wide_window() {
    let f = Fixture::new();
    let s = open_global_settings(&f);
    assert_eq!(s.screen(), Screen::Settings(SettingsTarget::Global));
    assert!(s.two_pane());
    assert_eq!(s.focus(), ids::settings_nav(0));
}

#[test]
fn the_rows_follow_the_section_list() {
    let f = Fixture::new();
    let mut s = open_global_settings(&f);
    press(&mut s, &f, &[go(Down)]);
    assert_eq!(s.settings_section(), 1, "Controller");
    press(&mut s, &f, &[Confirm]);
    assert_eq!(ids::row_of(s.focus()), Some((1, 0)), "A on a section enters its rows");
    press(&mut s, &f, &[go(Left)]);
    assert_eq!(ids::nav_section(s.focus()), Some(1));
}

#[test]
fn toggles_report_the_change_and_remember_it() {
    let f = Fixture::new();
    let mut s = open_global_settings(&f);
    let controller = section_index(&f, &s, SettingsTarget::Global, "controller");
    s.focus = ids::settings_nav(controller);
    press(&mut s, &f, &[Confirm, go(Down)]); // Controller > Vibration
    let fx = press(&mut s, &f, &[Confirm]);
    assert_eq!(fx, vec![Effect::Setting(SettingChange { app: None, key: "rumble", value: SettingValue::Bool(false) })]);
    assert!(!s.values().toggle(SettingsTarget::Global, "rumble", true));
}

#[test]
fn a_choice_opens_a_picker_and_the_interface_choice_sets_the_mode() {
    let f = Fixture::new();
    let mut s = open_global_settings(&f);
    press(&mut s, &f, &[Confirm, Confirm]); // into the rows, then the first row (Interface mode)
    assert_eq!(s.overlay(), Overlay::Menu(MenuPurpose::Choice(SettingsTarget::Global, "interface_mode")));
    assert_eq!(s.menu().unwrap().levels()[0].title, "Interface");
    let fx = press(&mut s, &f, &[go(Down), go(Down), Confirm]);
    assert_eq!(
        fx,
        vec![
            Effect::Setting(SettingChange { app: None, key: "interface_mode", value: SettingValue::Choice(2) }),
            Effect::SetMode(ModePref::Deck),
        ]
    );
    assert_eq!(s.overlay(), Overlay::None);
    assert_eq!(s.values().choice(SettingsTarget::Global, "interface_mode", 0), 2);
}

#[test]
fn a_picker_opens_on_the_current_choice_and_back_leaves_it_unchanged() {
    let f = Fixture::new();
    let mut s = open_global_settings(&f);
    s.values_mut().set(SettingsTarget::Global, "interface_mode", SettingValue::Choice(1));
    press(&mut s, &f, &[Confirm, Confirm]);
    assert_eq!(s.menu().unwrap().levels()[0].focus, 1);
    assert!(press(&mut s, &f, &[Back]).is_empty());
    assert_eq!(s.values().choice(SettingsTarget::Global, "interface_mode", 0), 1);
}

#[test]
fn a_narrow_window_drills_down_and_back_up() {
    let f = Fixture::new();
    let mut s = f.state();
    s.set_window((390., 780.), &f.view());
    press(&mut s, &f, &[MainMenu, go(Down), go(Down), go(Down), go(Down), Confirm]);
    assert!(!s.two_pane());
    assert_eq!(s.focus(), ids::settings_nav(0));
    press(&mut s, &f, &[go(Down), Confirm]);
    assert!(s.drilled());
    assert_eq!(ids::row_of(s.focus()), Some((1, 0)), "inside Controller");
    press(&mut s, &f, &[Back]);
    assert!(!s.drilled());
    assert_eq!(ids::nav_section(s.focus()), Some(1), "back on the list, same section");
    press(&mut s, &f, &[Back]);
    assert_eq!(s.screen(), Screen::Home);
}

#[test]
fn properties_open_from_options_and_back_returns_to_the_game_page() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Confirm]); // Game page of Starfall 64
    press(&mut s, &f, &[Options, go(Down), go(Down), Confirm]);
    assert_eq!(s.screen(), Screen::Settings(SettingsTarget::App(1)));
    press(&mut s, &f, &[Back]);
    assert_eq!(s.screen(), Screen::Game(1));
}

#[test]
fn the_uninstall_row_asks_before_acting() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options, go(Down), go(Down), Confirm]); // Properties
    // Installed files holds the Uninstall row in its Remove group (row index 4).
    let files = section_index(&f, &s, SettingsTarget::App(1), "files");
    s.focus = ids::settings_nav(files);
    press(&mut s, &f, &[Confirm]);
    s.focus = ids::settings_row(files, 4);
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.overlay(), Overlay::Confirm(ConfirmKind::Uninstall(1)));
}

#[test]
fn launch_options_are_a_text_row() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options, go(Down), go(Down), Confirm, Confirm]);
    assert_eq!(ids::row_of(s.focus()), Some((0, 0)));
    s.focus = ids::settings_row(0, 1);
    assert_eq!(press(&mut s, &f, &[Confirm]), vec![Effect::BeginTextEntry(TextField::LaunchOptions)]);
}

/// The position of a section in a target's schema, by its id, so a test does not depend on how many
/// sections come before it.
fn section_index(f: &Fixture, s: &DeckState, target: SettingsTarget, id: &str) -> usize {
    let schema = s.settings_schema(target, &f.view()).expect("schema");
    schema.sections.iter().position(|section| section.id == id).unwrap_or_else(|| panic!("no section {id}"))
}

/// The (section, row) of a launch row in a target's schema.
fn launch_row(f: &Fixture, s: &DeckState, target: SettingsTarget, key: reclaw_games::settings::SettingKey) -> Option<(usize, usize)> {
    let schema = s.settings_schema(target, &f.view())?;
    schema
        .sections
        .iter()
        .enumerate()
        .find_map(|(si, section)| section.rows().position(|r| matches!(r.kind, RowKind::Launch { key: k } if k == key)).map(|ri| (si, ri)))
}

#[test]
fn only_a_game_that_declares_launch_settings_gets_a_display_section() {
    let f = Fixture::new();
    let s = f.state();
    let sections = |target| s.settings_schema(target, &f.view()).expect("schema").sections.iter().map(|x| x.id).collect::<Vec<_>>();
    assert!(sections(SettingsTarget::App(1)).contains(&"display"), "Starfall 64 declares some");
    assert!(!sections(SettingsTarget::App(3)).contains(&"display"), "Kart Ruins declares none, so there is nothing to show");
    assert!(sections(SettingsTarget::Global).contains(&"games"), "the defaults are always on the global page");
}

#[test]
fn a_launch_row_opens_a_picker_and_choosing_sends_the_setting() {
    use reclaw_games::settings::{SettingKey, SettingValue as Launch};
    let f = Fixture::new();
    let mut s = f.state();
    s.open_settings(SettingsTarget::App(1), &f.view());
    let (section, row) = launch_row(&f, &s, SettingsTarget::App(1), SettingKey::Vsync).expect("Starfall 64 offers vsync");
    s.focus = ids::settings_row(section, row);
    s.settings_section = section;
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.overlay(), Overlay::Menu(MenuPurpose::Launch(SettingsTarget::App(1), SettingKey::Vsync)));
    let labels: Vec<String> = s.menu().expect("menu").levels()[0].entries.iter().map(|e| e.label.clone()).collect();
    assert_eq!(labels, vec!["Game's own", "On", "Off"]);
    // Down to "Off", confirm.
    let fx = press(&mut s, &f, &[go(Down), go(Down), Confirm]);
    assert_eq!(fx, vec![Effect::LaunchSetting { app: Some(1), key: SettingKey::Vsync, value: Some(Launch::Bool(false)) }]);
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn choosing_the_first_entry_clears_the_choice() {
    use reclaw_games::settings::{SettingKey, SettingValue as Launch};
    let mut f = Fixture::new();
    f.launch.apps.entry(1).or_default().set(SettingKey::Vsync, Launch::Bool(false));
    let mut s = f.state();
    s.open_settings(SettingsTarget::App(1), &f.view());
    let (section, row) = launch_row(&f, &s, SettingsTarget::App(1), SettingKey::Vsync).expect("offered");
    s.focus = ids::settings_row(section, row);
    s.settings_section = section;
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.menu().expect("menu").levels()[0].focus, 2, "opens on the current choice, Off");
    let fx = press(&mut s, &f, &[go(Up), go(Up), Confirm]);
    assert_eq!(fx, vec![Effect::LaunchSetting { app: Some(1), key: SettingKey::Vsync, value: None }]);
}
