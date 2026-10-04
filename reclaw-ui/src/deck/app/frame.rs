use freya::prelude::*;
use reclaw_input::{ActionMap, ControllerInfo, ControllerKind, FocusId};

use super::text_boxes::TextBoxes;
use crate::{
    activity::Activity,
    deck::{
        DeckState, LastInput,
        settings::{Schema, SettingsTarget},
    },
    model::GameEntry,
};

/// A settings page's content: which page, its schema, and the text on its launch rows.
#[derive(Clone, PartialEq)]
pub struct SettingsShown {
    pub target: SettingsTarget,
    pub schema: Schema,
    pub launch_text: std::collections::HashMap<reclaw_games::settings::SettingKey, String>,
}

/// Everything one render needs, gathered once so the page and overlay builders are plain
/// functions of a single value.
#[derive(Clone)]
pub struct Frame {
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
    /// The schema of the Settings page being shown, or of the one shown last.
    pub settings: Option<SettingsShown>,
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
