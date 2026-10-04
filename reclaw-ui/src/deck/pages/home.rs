use std::{collections::HashMap, rc::Rc};

use freya::prelude::*;
use reclaw_input::FocusId;

use crate::{
    activity::Indicator,
    deck::{DeckView, NowPlayingBanner, Shelf, ShelfSpec, ids, shelf_top, shelves},
    prelude::*,
};

/// Library home: Now Playing banner (when an app is active) over the "Continue" and "All apps"
/// shelves. The page snaps vertically to the focused shelf, like Big Picture.
#[derive(Clone, PartialEq)]
pub struct HomePage {
    games: Vec<GameEntry>,
    indicators: Rc<HashMap<u32, Indicator>>,
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
        Self { games, indicators, focus, ring_visible, viewport_w, on_click }
    }
}

impl Component for HomePage {
    fn render(&self) -> impl IntoElement {
        let view = DeckView { games: &self.games, downloads: &[], launch: None, notices: None };
        let active = view.active_game().cloned();
        let banner = active.is_some();
        let specs: Vec<ShelfSpec> = shelves(&view);

        let focused_shelf = ids::tile_shelf(self.focus);
        let offset_y = match focused_shelf {
            Some(i) if i > 0 => shelf_top(i, banner),
            _ => 0.,
        };

        let shelf_elements = specs.iter().enumerate().map(|(i, spec)| {
            let games: Vec<GameEntry> = spec.games.iter().filter_map(|id| self.games.iter().find(|g| g.id == *id).cloned()).collect();
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
