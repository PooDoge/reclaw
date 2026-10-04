//! The form-like pages: Install, and the Settings/Properties pages with text entry.
use super::support::*;
use crate::deck::settings::{SettingChange, SettingValue, SettingsTarget, TextField};

fn open_install(f: &Fixture) -> DeckState {
    let mut s = f.state();
    s.click(tile(1, 3), &f.view()); // Kart Ruins: needs the user's game file
    s.apply(Confirm, &f.view());
    s
}

#[test]
fn install_is_a_page_not_a_popup() {
    let f = Fixture::new();
    let s = open_install(&f);
    assert_eq!(s.screen(), Screen::Install(3));
    assert_eq!(s.focus(), ids::INSTALL_LOCATION);
    assert!(!s.install_draft().can_submit());
}

#[test]
fn install_needs_the_users_own_file_before_it_can_submit() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    // The footer buttons sit under the fields; Down from the last field lands on Install (nearest
    // by center), Left from there on Cancel.
    press(&mut s, &f, &[go(Down), go(Down), go(Down), go(Down)]);
    assert_eq!(s.focus(), ids::INSTALL_SUBMIT);
    assert!(press(&mut s, &f, &[Confirm]).is_empty(), "no file chosen yet");
    assert_eq!(s.screen(), Screen::Install(3));

    press(&mut s, &f, &[go(Up), go(Up), go(Up)]);
    assert_eq!(s.focus(), ids::INSTALL_FILE);
    assert_eq!(press(&mut s, &f, &[Confirm]), vec![Effect::ChooseFile(3)]);
    s.set_install_file("~/Games/kart.z64");

    press(&mut s, &f, &[go(Down), go(Down), go(Down)]);
    assert_eq!(s.focus(), ids::INSTALL_SUBMIT);
    assert_eq!(press(&mut s, &f, &[Confirm]), vec![Effect::SubmitInstall(3)]);
    assert_eq!(s.screen(), Screen::Game(3), "back on the game page");
}

#[test]
fn cancel_is_left_of_install() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    press(&mut s, &f, &[go(Down), go(Down), go(Down), go(Down), go(Left)]);
    assert_eq!(s.focus(), ids::INSTALL_CANCEL);
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.screen(), Screen::Game(3));
}

#[test]
fn install_switches_toggle_in_place() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    press(&mut s, &f, &[go(Down), go(Down), Confirm, go(Down), Confirm]);
    assert!(!s.install_draft().shortcut, "shortcut was on by default");
    assert!(s.install_draft().prerelease);
}

#[test]
fn cancel_discards_the_draft() {
    let f = Fixture::new();
    let mut s = open_install(&f);
    s.set_install_file("x");
    press(&mut s, &f, &[Back]);
    s.apply(Confirm, &f.view());
    assert_eq!(s.screen(), Screen::Install(3));
    assert!(!s.install_draft().can_submit(), "a fresh draft each time");
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
    press(&mut s, &f, &[go(Down), go(Down), go(Down), go(Down)]);
    assert_eq!(s.focus(), ids::INSTALL_SUBMIT);
    assert_eq!(s.reveal_target(&f.view()), None);
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
    press(&mut s, &f, &[go(Down), Confirm, go(Down)]); // Controller > Vibration
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
    press(&mut s, &f, &[Options, go(Down), go(Down), go(Down), Confirm]);
    assert_eq!(s.screen(), Screen::Settings(SettingsTarget::App(1)));
    press(&mut s, &f, &[Back]);
    assert_eq!(s.screen(), Screen::Game(1));
}

#[test]
fn the_uninstall_row_asks_before_acting() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options, go(Down), go(Down), go(Down), Confirm]); // Properties
    // Installed files is the fourth section; its Remove group holds the Uninstall row (index 4).
    press(&mut s, &f, &[go(Down), go(Down), go(Down), Confirm]);
    s.focus = ids::settings_row(3, 4);
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.overlay(), Overlay::Confirm(ConfirmKind::Uninstall(1)));
}

#[test]
fn launch_options_are_a_text_row() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options, go(Down), go(Down), go(Down), Confirm, Confirm]);
    assert_eq!(ids::row_of(s.focus()), Some((0, 0)));
    s.focus = ids::settings_row(0, 1);
    assert_eq!(press(&mut s, &f, &[Confirm]), vec![Effect::BeginTextEntry(TextField::LaunchOptions)]);
}
