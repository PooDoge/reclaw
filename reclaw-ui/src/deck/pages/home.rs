use std::{collections::HashMap, rc::Rc};

use freya::prelude::*;
use reclaw_input::FocusId;

use crate::{
    activity::Indicator,
    deck::{DeckView, NowPlayingBanner, Section, Shelf, ShelfSpec, ids, shelf_top, shelves_of},
    prelude::*,
    systems::Sort,
};

/// A tab of shelves. The Library: Now Playing banner (when an app is active) over the "Continue" and
/// "Not installed" shelves. The Catalog: a shelf per system. The page snaps vertically to the focused
/// shelf, like Big Picture.
#[derive(Clone, PartialEq)]
pub struct HomePage {
    section: Section,
    games: Vec<GameEntry>,
    catalog: Vec<GameEntry>,
    indicators: Rc<HashMap<u32, Indicator>>,
    sort: Sort,
    focus: FocusId,
    ring_visible: bool,
    viewport_w: f32,
    on_click: EventHandler<FocusId>,
}

impl HomePage {
    pub fn new(
        games: Vec<GameEntry>,
        indicators: Rc<HashMap<u32, Indicator>>,
        focus: FocusId,
        ring_visible: bool,
        viewport_w: f32,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self {
            section: Section::Library,
            games,
            catalog: Vec::new(),
            indicators,
            sort: Sort::default(),
            focus,
            ring_visible,
            viewport_w,
            on_click,
        }
    }

    /// Show the Catalog tab's shelves, drawn from `catalog`, instead of the Library's.
    pub fn catalog(mut self, catalog: Vec<GameEntry>) -> Self {
        self.section = Section::Catalog;
        self.catalog = catalog;
        self
    }

    /// How the shelves are ordered; by system there is one per system.
    pub fn sort(mut self, sort: Sort) -> Self {
        self.sort = sort;
        self
    }
}

impl Component for HomePage {
    fn render(&self) -> impl IntoElement {
        let view = DeckView {
            games: &self.games,
            catalog: &self.catalog,
            downloads: &[],
            launch: None,
            notices: None,
            sort: self.sort,
            recents: &[],
        };
        let active = view.active_game().filter(|_| self.section == Section::Library).cloned();
        let banner = active.is_some();
        let specs: Vec<ShelfSpec> = shelves_of(&view, self.section);

        let focused_shelf = ids::tile_shelf(self.focus);
        let offset_y = match focused_shelf {
            Some(i) if i > 0 => shelf_top(i, banner),
            _ => 0.,
        };

        let shelf_elements = specs.iter().enumerate().map(|(i, spec)| {
            let games: Vec<GameEntry> = spec.games.iter().filter_map(|id| view.game(*id).cloned()).collect();
            Shelf::new(spec.title, i, games, self.focus, self.ring_visible, self.viewport_w, self.on_click.clone())
                .indicators(self.indicators.clone())
                .key(i)
                .into_element()
        });

        rect().width(Size::fill()).height(Size::fill()).overflow(Overflow::Clip).child(
            rect()
                .vertical()
                .width(Size::fill())
                .offset_y(-offset_y)
                .padding(Gaps::new(8., 0., 0., 0.))
                .maybe_child(active.map(|game| {
                    rect().padding(Gaps::new(0., 0., 24., 0.)).child(NowPlayingBanner::new(
                        game,
                        self.focus,
                        self.ring_visible,
                        ids::BANNER_RESUME,
                        ids::BANNER_STOP,
                        self.on_click.clone(),
                    ))
                }))
                .children(shelf_elements),
        )
    }
}
