//! What the reducer reads from the host (the game list, the catalog, the queue) and the shelf
//! geometry of the Library and Catalog tabs derived from it.
use reclaw_input::Rect;

use crate::{
    activity::Activity,
    metrics::*,
    model::*,
    nav::Section,
    notices::Notices,
    settings::LaunchContext,
    systems::{self, Sort},
};

pub struct DeckView<'a> {
    pub games: &'a [GameEntry],
    /// Every catalog project, as its library entry when the user has added it (`catalog::catalog_entries`).
    /// Empty where a view is only used for the Library.
    pub catalog: &'a [GameEntry],
    /// The Downloads list: running jobs first, then failed, then finished.
    pub downloads: &'a [Activity],
    /// What launch-setting rows need (the display, the catalog, the saved choices). `None` where a
    /// view is only used for the Home shelves.
    pub launch: Option<LaunchContext<'a>>,
    /// The notifications waiting; `None` where a view does not need them.
    pub notices: Option<&'a Notices>,
    /// How Home orders its shelves; by system it makes one shelf per system.
    pub sort: Sort,
    /// The pages visited lately, newest first, without the one showing: the Quick access panel lists
    /// the first few. The router keeps them; this is a copy for the moment.
    pub recents: &'a [crate::nav::Route],
}

impl DeckView<'_> {
    /// A game by id: the library's entry, or the catalog's for a project the user has not added.
    pub fn game(&self, id: u32) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.id == id).or_else(|| self.catalog.iter().find(|g| g.id == id))
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

fn ids(games: &[GameEntry]) -> Vec<u32> {
    games.iter().map(|g| g.id).collect()
}

/// The Library tab: "Continue" (installed or active) then "Not installed", in the view's sort order. A
/// game is on one shelf only, and an empty shelf is left out. Sorted by system, "Not installed" becomes one
/// shelf per system, each titled with the system.
pub fn shelves(view: &DeckView) -> Vec<ShelfSpec> {
    let (cont, rest): (Vec<GameEntry>, Vec<GameEntry>) =
        systems::sorted(view.games.to_vec(), view.sort).into_iter().partition(|g| g.status.is_installed() || g.run.is_active());
    let mut out = Vec::new();
    if !cont.is_empty() {
        out.push(ShelfSpec { title: "Continue", games: ids(&cont) });
    }
    if view.sort == Sort::System {
        out.extend(systems::grouped(&rest).into_iter().map(|(platform, games)| ShelfSpec { title: platform.label(), games: ids(&games) }));
    } else if !rest.is_empty() {
        out.push(ShelfSpec { title: "Not installed", games: ids(&rest) });
    }
    out
}

/// The Catalog tab: a shelf per system, alphabetical within each, like the desktop Catalog's platform chips.
pub fn catalog_shelves(view: &DeckView) -> Vec<ShelfSpec> {
    systems::grouped(view.catalog).into_iter().map(|(platform, games)| ShelfSpec { title: platform.label(), games: ids(&games) }).collect()
}

/// The shelves of a tab that has them (Library, Catalog); none for the others.
pub fn shelves_of(view: &DeckView, section: Section) -> Vec<ShelfSpec> {
    match section {
        Section::Library => shelves(view),
        Section::Catalog => catalog_shelves(view),
        Section::Downloads | Section::Mods => Vec::new(),
    }
}

/// How many recent pages the Quick access panel lists.
pub const QA_RECENTS: usize = 4;

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
