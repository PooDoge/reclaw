//! State behind the install dialog, created once per screen.
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub struct InstallForm {
    pub location: State<String>,
    pub game_file: State<Option<String>>,
    pub shortcut: State<bool>,
    pub prerelease: State<bool>,
}

impl InstallForm {
    pub fn use_new() -> Self {
        Self {
            location: use_state(|| String::from("~/Reclaw/Apps")),
            game_file: use_state(|| None::<String>),
            shortcut: use_state(|| true),
            prerelease: use_state(|| false),
        }
    }
}
