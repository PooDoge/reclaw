use std::{collections::HashMap, rc::Rc};

use freya::prelude::*;
use reclaw_input::{ActionMap, Button, ControllerInfo, ControllerKind, FocusId};

use super::text_boxes::TextBoxes;
use crate::{
    activity::{Activity, Indicator},
    deck::{
        DeckState, LastInput,
        settings::{Schema, SettingsTarget},
    },
    model::GameEntry,
    notices::Notice,
};

/// A settings page's content: which page, its schema, and the text on its launch rows.
#[derive(Clone, PartialEq)]
pub struct SettingsShown {
    pub target: SettingsTarget,
    pub schema: Schema,
    pub launch_text: std::collections::HashMap<reclaw_games::settings::SettingKey, String>,
    pub credentials: crate::credentials::CredentialsStatus,
}

/// Everything one render needs, gathered once so the page and overlay builders are plain
/// functions of a single value.
#[derive(Clone)]
pub struct Frame {
    pub state: DeckState,
    pub games: Vec<GameEntry>,
    pub downloads: Vec<Activity>,
    /// What the Quick access panel lists as pages to jump back to, by title.
    pub recents: Vec<String>,
    /// How Home orders its shelves.
    pub sort: crate::systems::Sort,
    /// What each game's card shows of its background work, by game id.
    pub indicators: Rc<HashMap<u32, Indicator>>,
    /// The toast on screen, if any: the newest notice, when nothing else is in the way.
    pub toast: Option<Notice>,
    /// The notice whose details are open, if any.
    pub notice_details: Option<Notice>,
    /// The button being held for the toast's ring.
    pub holding: Option<Button>,
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
