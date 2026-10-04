use freya::prelude::*;
use reclaw_input::FocusId;

use super::tile::DeckTile;
use crate::deck::{SHELF_TITLE_BLOCK, ids, layout::shelf_offset};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// A titled, horizontally scrolling row of tiles. The focused tile is kept centered by `offset_x`.
#[derive(Clone, PartialEq)]
pub struct Shelf {
    key: DiffKey,
    title: &'static str,
    index: usize,
    games: Vec<GameEntry>,
    focus: FocusId,
    ring_visible: bool,
    viewport_w: f32,
    on_click: EventHandler<FocusId>,
}

impl KeyExt for Shelf {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Shelf {
    pub fn new(
        title: &'static str,
        index: usize,
        games: Vec<GameEntry>,
        focus: FocusId,
        ring_visible: bool,
        viewport_w: f32,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self { key: DiffKey::None, title, index, games, focus, ring_visible, viewport_w, on_click }
    }
}

impl Component for Shelf {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let focused_idx = self.games.iter().position(|g| ids::tile(self.index, g.id) == self.focus);
        let offset = focused_idx.map_or(0., |i| shelf_offset(self.games.len(), i, self.viewport_w));

        let tiles = self.games.iter().enumerate().map(|(i, game)| {
            let id = ids::tile(self.index, game.id);
            let on_click = self.on_click.clone();
            DeckTile::new(game.clone(), self.ring_visible && focused_idx == Some(i))
                .on_press(move |_| on_click.call(id))
                .key(id.0)
                .into_element()
        });

        rect()
            .vertical()
            .width(Size::fill())
            .child(rect().height(Size::px(SHELF_TITLE_BLOCK)).child(TypeStyle::DeckHeading.text(self.title, t.ink)))
            .child(
                // Padding inside the clip gives the focus scale and glow room to draw.
                rect()
                    .width(Size::fill())
                    .overflow(Overflow::Clip)
                    .padding(Gaps::new(DECK_TILE_GAP, DECK_TILE_GAP, 0., DECK_TILE_GAP))
                    .child(rect().horizontal().spacing(DECK_TILE_GAP).offset_x(-offset).children(tiles)),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
