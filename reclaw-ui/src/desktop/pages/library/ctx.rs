//! What the three Library layouts share: the derived lists, the frame's state and the window facts.
//! Built once per render by `LibraryPage`.
use freya::prelude::*;

use super::filter::Filter;
use crate::{
    desktop::{DesktopEnv, GameDialogs},
    effect::Effect,
    nav::Nav,
    prelude::*,
};

pub(super) struct Ctx {
    /// Tokens, read once in `LibraryPage::render`: hooks must not run inside the layout helpers,
    /// which differ between layout classes and renders.
    pub t: Reclaw,
    pub env: DesktopEnv,
    pub games: Vec<GameEntry>,
    pub downloads: Vec<Download>,
    /// After the filter chips and the search box.
    pub visible: Vec<GameEntry>,
    pub current: Option<GameEntry>,
    pub installed: u32,
    pub updates: u32,
    pub selected: State<Option<u32>>,
    pub filter: State<Filter>,
    pub search: State<String>,
    pub dialogs: GameDialogs,
    pub nav: Nav,
    pub on_effect: EventHandler<Effect>,
    pub toggle_deck: EventHandler<()>,
}
