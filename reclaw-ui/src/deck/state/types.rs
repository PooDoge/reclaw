//! Plain data of Deck state: where we are, what was asked of the host.

pub use crate::deck::settings::{SettingChange, SettingValue, SettingsTarget, TextField};

pub use crate::nav::Section;

/// A full page. Dialogs that take input are pages here (`Install`), as are settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Home,
    Game(u32),
    Install(u32),
    Settings(SettingsTarget),
}

/// Why a menu is open; decides what its choices mean.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuPurpose {
    /// The Options menu of one app.
    Options(u32),
    /// A picker for a choice row: which target and key.
    Choice(SettingsTarget, &'static str),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ConfirmKind {
    Uninstall(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Overlay {
    None,
    MainMenu,
    QuickAccess,
    Menu(MenuPurpose),
    Confirm(ConfirmKind),
}

pub use crate::app_menu::MenuAction;

/// Drives which hint glyphs show and whether the focus ring is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LastInput {
    Gamepad(reclaw_input::ControllerKind),
    Keyboard,
    Pointer,
}

pub use crate::effect::{Effect, ModePref};

/// Entries of the main menu, top to bottom.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MainMenuEntry {
    Section(Section),
    Settings,
    SwitchToDesktop,
}

pub const MAIN_MENU: [MainMenuEntry; 6] = [
    MainMenuEntry::Section(Section::Library),
    MainMenuEntry::Section(Section::Catalog),
    MainMenuEntry::Section(Section::Downloads),
    MainMenuEntry::Section(Section::Mods),
    MainMenuEntry::Settings,
    MainMenuEntry::SwitchToDesktop,
];
