use freya::prelude::*;
use reclaw_input::{ActionMap, ControllerInfo, ControllerKind, FocusId};

use super::text_boxes::TextBoxes;
use crate::{
    activity::Activity,
    deck::{DeckState, LastInput, settings::Schema},
    model::GameEntry,
};

/// Everything one render needs, gathered once so the screen and overlay builders are plain
/// functions of a single value.
#[derive(Clone)]
pub(super) struct Frame {
    pub state: DeckState,
    pub games: Vec<GameEntry>,
    pub downloads: Vec<Activity>,
    pub pad: Option<ControllerInfo>,
    pub kind: ControllerKind,
    /// Last input with the connected pad's kind filled in, for glyph choice.
    pub last_input: LastInput,
    pub ring: bool,
    pub window: (f32, f32),
    pub keyboard_inset: f32,
    pub map: ActionMap,
    pub texts: TextBoxes,
    /// The settings schema of the open Settings page, if any.
    pub schema: Option<Schema>,
    /// The text on each launch row of that schema.
    pub launch_text: std::collections::HashMap<reclaw_games::settings::SettingKey, String>,
    /// The focused field's span in the page body, to keep above the keyboard.
    pub reveal: Option<(f32, f32)>,
    pub click: EventHandler<FocusId>,
    pub back: EventHandler<()>,
    pub dismiss: EventHandler<()>,
    pub pick: EventHandler<(usize, usize)>,
}

impl Frame {
    pub fn game(&self, id: u32) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.id == id)
    }
}
