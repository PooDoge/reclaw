//! Read one part of the store. Each hook subscribes the calling component to the channel that part
//! lives on, so it redraws when that part changes and not otherwise. Hooks: call them at the top of a
//! component, unconditionally, and pass the values down.
use freya::radio::*;
use reclaw_config::{LaunchPrefs, WindowPrefs};
use reclaw_games::{project::ProjectInfo, settings::DisplayEnvironment};
use reclaw_input::ControllerInfo;

use super::{
    channel::AppChannel,
    state::{AppState, Mailbox},
    status::CatalogStatus,
};
use crate::{
    activity::ActivityBoard,
    model::{GameEntry, ModEntry},
    notices::Notices,
    settings::SettingsValues,
};

/// A subscription for an effect to read: `use_side_effect` re-runs when this channel changes, if the
/// effect reads through the returned handle. The hooks below are for rendering; this is for the
/// rare effect that must react to the store.
pub fn use_channel(channel: AppChannel) -> Radio<AppState, AppChannel> {
    use_radio::<AppState, AppChannel>(channel)
}

fn read<T>(channel: AppChannel, pick: impl FnOnce(&AppState) -> T) -> T {
    let radio = use_radio::<AppState, AppChannel>(channel);
    let state = radio.read();
    pick(&state)
}

pub fn use_games() -> Vec<GameEntry> {
    read(AppChannel::Games, |s| s.games.clone())
}

pub fn use_projects() -> Vec<ProjectInfo> {
    read(AppChannel::Projects, |s| s.projects.clone())
}

pub fn use_catalog_status() -> CatalogStatus {
    read(AppChannel::Catalog, |s| s.catalog.clone())
}

pub fn use_mods() -> Vec<ModEntry> {
    read(AppChannel::Mods, |s| s.mods.clone())
}

/// All background work. Redraws on every progress tick of any job; a card for one game should use
/// [`use_activity_of`].
pub fn use_activity() -> ActivityBoard {
    read(AppChannel::Activity, |s| s.activity.clone())
}

/// The board, redrawn only when this game's jobs change.
pub fn use_activity_of(game_id: u32) -> ActivityBoard {
    read(AppChannel::ActivityOf(game_id), |s| s.activity.clone())
}

pub fn use_notices() -> Notices {
    read(AppChannel::Notices, |s| s.notices.clone())
}

pub fn use_settings() -> SettingsValues {
    read(AppChannel::Settings, |s| s.settings.clone())
}

pub fn use_launch() -> LaunchPrefs {
    read(AppChannel::Launch, |s| s.launch.clone())
}

pub fn use_window_prefs() -> WindowPrefs {
    read(AppChannel::Window, |s| s.window.clone())
}

pub fn use_controller() -> Option<ControllerInfo> {
    read(AppChannel::Controller, |s| s.controller.clone())
}

pub fn use_keyboard_inset() -> f32 {
    read(AppChannel::Keyboard, |s| s.keyboard_inset)
}

pub fn use_display() -> DisplayEnvironment {
    read(AppChannel::Display, |s| s.display.clone())
}

pub fn use_mailbox() -> Mailbox {
    read(AppChannel::Mailbox, |s| s.mailbox.clone())
}
