//! What the three Library layouts share: the derived lists, the frame's state and the window facts.
//! Built once per render by `LibraryPage`.
use freya::prelude::*;
use reclaw_games::project::Platform;

use super::filter::Filter;
use crate::{
    activity::{ActivityBoard, SidebarEntry},
    desktop::{DesktopEnv, GameDialogs},
    effect::Effect,
    nav::Nav,
    prelude::*,
    systems::Sort,
};

pub(super) struct Ctx {
    /// Tokens, read once in `LibraryPage::render`: hooks must not run inside the layout helpers,
    /// which differ between layout classes and renders.
    pub t: Reclaw,
    pub env: DesktopEnv,
    pub games: Vec<GameEntry>,
    pub activity: ActivityBoard,
    /// Games with an update waiting, work in progress, or a result from this run.
    pub updates_section: Vec<SidebarEntry>,
    /// After the filter chips and the search box.
    pub visible: Vec<GameEntry>,
    pub current: Option<GameEntry>,
    pub installed: u32,
    pub updates: u32,
    pub selected: State<Option<u32>>,
    pub filter: State<Filter>,
    /// The system filter, and the systems the library has games for (with a count each) to offer.
    pub system: State<Option<Platform>>,
    pub systems: Vec<(Platform, u32)>,
    pub sort: Sort,
    pub search: State<String>,
    pub dialogs: GameDialogs,
    pub nav: Nav,
    pub on_effect: EventHandler<Effect>,
    pub toggle_deck: EventHandler<()>,
}
