//! What the reducer reads from the host (the game list, the queue) and the Home shelf geometry
//! derived from it.
use reclaw_input::Rect;

use crate::{activity::Activity, metrics::*, model::*, notices::Notices, settings::LaunchContext};

pub struct DeckView<'a> {
    pub games: &'a [GameEntry],
    /// The Downloads list: running jobs first, then failed, then finished.
    pub downloads: &'a [Activity],
    /// What launch-setting rows need (the display, the catalog, the saved choices). `None` where a
    /// view is only used for the Home shelves.
    pub launch: Option<LaunchContext<'a>>,
    /// The notifications waiting; `None` where a view does not need them.
    pub notices: Option<&'a Notices>,
}

impl DeckView<'_> {
    pub fn game(&self, id: u32) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.id == id)
    }

    /// The notification on screen, if any: the newest.
    pub fn top_notice(&self) -> Option<&crate::notices::Notice> {
        self.notices.and_then(Notices::top)
    }

    /// The app shown in the Now Playing banner: the first one that is starting, running or stopping.
    pub fn active_game(&self) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.run.is_active())
    }
}

pub struct ShelfSpec {
    pub title: &'static str,
    pub games: Vec<u32>,
}

/// "Continue" (installed or active) then "All apps". An empty Continue shelf is left out.
pub fn shelves(view: &DeckView) -> Vec<ShelfSpec> {
    let cont: Vec<u32> = view.games.iter().filter(|g| g.status.is_installed() || g.run.is_active()).map(|g| g.id).collect();
    let mut out = Vec::new();
    if !cont.is_empty() {
        out.push(ShelfSpec { title: "Continue", games: cont });
    }
    out.push(ShelfSpec { title: "All apps", games: view.games.iter().map(|g| g.id).collect() });
    out
}

pub const BANNER_H: f32 = 72.;
pub const BANNER_BLOCK: f32 = BANNER_H + 24.;
pub const SHELF_TITLE_BLOCK: f32 = 44.;
pub const SHELF_H: f32 = SHELF_TITLE_BLOCK + DECK_TILE_H + DECK_CAPTION_H + 32.;

pub fn shelf_top(index: usize, banner: bool) -> f32 {
    (if banner { BANNER_BLOCK } else { 0. }) + index as f32 * SHELF_H
}

pub fn tile_rect(shelf: usize, tile: usize, banner: bool) -> Rect {
    Rect::new(
        tile as f32 * (DECK_TILE_W + DECK_TILE_GAP),
        shelf_top(shelf, banner) + SHELF_TITLE_BLOCK,
        DECK_TILE_W,
        DECK_TILE_H + DECK_CAPTION_H,
    )
}
