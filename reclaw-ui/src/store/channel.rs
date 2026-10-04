use freya::radio::RadioChannel;

use super::state::AppState;

/// What a reader subscribes to. A write notifies the channels it touched and no others, so a
/// download's progress does not redraw the settings page.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AppChannel {
    Games,
    Projects,
    Mods,
    /// Any activity changed.
    Activity,
    /// One game's activity changed. Notifies [`Activity`](Self::Activity) readers too.
    ActivityOf(u32),
    Notices,
    /// Settings values (toggles, choices, texts).
    Settings,
    /// Launch defaults and overrides.
    Launch,
    Favorites,
    Window,
    Controller,
    Keyboard,
    Display,
    Mailbox,
    /// A channel nobody reads. Used to write without redrawing anything.
    Quiet,
}

impl AppChannel {
    /// Whether a change here must reach the settings file.
    pub fn is_persisted(self) -> bool {
        matches!(self, Self::Settings | Self::Launch | Self::Favorites | Self::Window)
    }
}

impl RadioChannel<AppState> for AppChannel {
    fn derive_channel(self, _state: &AppState) -> Vec<Self> {
        match self {
            Self::ActivityOf(_) => vec![self, Self::Activity],
            other => vec![other],
        }
    }
}
