//! The state the host application and the UI share. The host owns it (it creates it, and writes
//! to it as processes start and stop, pads connect and the OS keyboard moves); the UI reads it and
//! asks for changes through [`Effect`](crate::effect::Effect)s. Both interfaces take the same
//! `HostState`, so switching between them loses nothing.
use freya::prelude::*;
use reclaw_input::ControllerInfo;

use crate::model::{Download, GameEntry};

#[derive(Clone, Copy, PartialEq)]
pub struct HostState {
    pub games: State<Vec<GameEntry>>,
    pub downloads: State<Vec<Download>>,
    /// The connected pad, for glyph choice. `None` with no pad.
    pub controller: State<Option<ControllerInfo>>,
    /// Height in px of the on-screen keyboard, 0 when hidden. The OS binding writes it.
    pub keyboard_inset: State<f32>,
    /// The host's answer to `Effect::ChooseFile`: the path the user picked. The UI consumes it.
    pub chosen_file: State<Option<String>>,
}

impl HostState {
    /// Create the states in the calling component, once.
    pub fn use_new(games: Vec<GameEntry>, downloads: Vec<Download>) -> Self {
        Self {
            games: use_state(move || games),
            downloads: use_state(move || downloads),
            controller: use_state(|| None),
            keyboard_inset: use_state(|| 0.),
            chosen_file: use_state(|| None),
        }
    }
}
