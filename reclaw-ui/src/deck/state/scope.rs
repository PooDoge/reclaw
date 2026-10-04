//! A scope is one set of focus targets: the page you are on, or the overlay trapping focus.
//! Focus is remembered per scope, so Back returns you to where you were.
use super::types::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum SettingsPane {
    /// Section list and rows side by side; the section currently shown.
    TwoPane(usize),
    /// Narrow windows: the list of sections...
    List,
    /// ...and one section's rows.
    Detail(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Scope {
    Home(Section),
    Game(u32),
    Install(u32),
    Settings(SettingsTarget, SettingsPane),
    MainMenu,
    QuickAccess,
    /// A cascading menu: its own focus lives in `MenuState`, not in spatial nodes.
    Menu,
    Confirm(ConfirmKind),
    Notice(crate::notices::NoticeId),
}
