//! The state the host application and the UI share. The host owns it (it creates it, and writes
//! to it as processes start and stop, pads connect and the OS keyboard moves); the UI reads it and
//! asks for changes through [`Effect`](crate::effect::Effect)s. Both interfaces take the same
//! `HostState`, so switching between them loses nothing.
//!
//! * `changes`: the edits the UI owns (favorites, launch settings), as plain functions on plain data
//!
//! The `Shell` applies those edits with [`HostState::apply`] before passing the effect on, so the
//! screen reflects a choice at once and the host only has to persist it.
use freya::prelude::*;
use reclaw_input::ControllerInfo;

use std::collections::BTreeMap;

use reclaw_games::{
    project::ProjectInfo,
    settings::{DisplayEnvironment, SettingsLayer},
};

mod changes;

use crate::{
    effect::Effect,
    model::{Download, GameEntry, ModEntry},
    nav::Route,
};

#[derive(Clone, Copy, PartialEq)]
pub struct HostState {
    pub games: State<Vec<GameEntry>>,
    pub downloads: State<Vec<Download>>,
    /// What the catalog says about every project, installed or not.
    pub projects: State<Vec<ProjectInfo>>,
    pub mods: State<Vec<ModEntry>>,
    /// The connected pad, for glyph choice. `None` with no pad.
    pub controller: State<Option<ControllerInfo>>,
    /// Height in px of the on-screen keyboard, 0 when hidden. The OS binding writes it.
    pub keyboard_inset: State<f32>,
    /// The host's answer to `Effect::ChooseFile`: the path the user picked. The UI consumes it.
    pub chosen_file: State<Option<String>>,
    /// A page the host wants shown: a deep link, a notification that was clicked. The UI opens it and clears it.
    pub open: State<Option<Route>>,
    /// What the display offers, for launch settings. The host fills it from the windowing system.
    pub display: State<DisplayEnvironment>,
    /// The user's launch settings for every game, and per-game overrides keyed by game id.
    pub game_defaults: State<SettingsLayer>,
    pub game_overrides: State<BTreeMap<u32, SettingsLayer>>,
}

impl HostState {
    /// Create the states in the calling component, once.
    pub fn use_new(games: Vec<GameEntry>, downloads: Vec<Download>, projects: Vec<ProjectInfo>, mods: Vec<ModEntry>) -> Self {
        Self {
            games: use_state(move || games),
            downloads: use_state(move || downloads),
            projects: use_state(move || projects),
            mods: use_state(move || mods),
            controller: use_state(|| None),
            keyboard_inset: use_state(|| 0.),
            chosen_file: use_state(|| None),
            open: use_state(|| None),
            display: use_state(DisplayEnvironment::unknown),
            game_defaults: use_state(SettingsLayer::default),
            game_overrides: use_state(BTreeMap::new),
        }
    }
}

impl HostState {
    /// Apply the part of an effect that is the UI's own state. Everything else is the host's job and
    /// is ignored here. Call from an event handler, never during render.
    pub fn apply(&self, effect: &Effect) {
        match effect {
            Effect::ToggleFavorite(id) => {
                let mut games = self.games;
                changes::toggle_favorite(&mut games.write(), *id);
            }
            Effect::LaunchSetting { app, key, value } => {
                let (mut defaults, mut overrides) = (self.game_defaults, self.game_overrides);
                changes::set_launch_setting(&mut defaults.write(), &mut overrides.write(), *app, *key, value.clone());
            }
            Effect::InstallMod { provider, id } => {
                let mut mods = self.mods;
                changes::mark_mod_installing(&mut mods.write(), *provider, id);
            }
            _ => {}
        }
    }
}
