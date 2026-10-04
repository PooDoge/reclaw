use freya::prelude::*;

use super::focus::FocusFrame;
use crate::{components::PressHandler, metrics::*, prelude::*, typography::TypeStyle};

/// Big-art game tile. Title always shows; the badge appears when focused (or running), in space
/// that is reserved up front so moving focus never reflows the shelf.
#[derive(Clone, PartialEq)]
pub struct DeckTile {
    game: GameEntry,
    focused: bool,
    on_press: Option<PressHandler>,
    key: DiffKey,
}

impl KeyExt for DeckTile {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl DeckTile {
    pub fn new(game: GameEntry, focused: bool) -> Self {
        Self {
            game,
            focused,
            on_press: None,
            key: DiffKey::None,
        }
    }

    pub fn on_press(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_press = Some(handler.into());
        self
    }
}

impl Component for DeckTile {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let running = self.game.run.is_active();
        let show_badge = self.focused || running;

        let art = rect()
            .width(Size::px(DECK_TILE_W))
            .height(Size::px(DECK_TILE_H))
            .corner_radius(RADIUS_MD)
            .overflow(Overflow::Clip)
            .child(ArtPlaceholder::new(
                "CAPSULE 3:4",
                Size::fill(),
                Size::fill(),
            ));

        let badge = if running {
            StatusBadge::running()
        } else {
            StatusBadge::new(self.game.status)
        };
        let caption = rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::px(DECK_TILE_W))
            .height(Size::px(DECK_CAPTION_H))
            .padding(Gaps::new(SPACE_3, 0., 0., 0.))
            .child(
                TypeStyle::DeckLabel
                    .text(self.game.title.clone(), t.ink)
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis),
            )
            .maybe(show_badge, |el| el.child(badge));

        rect()
            .vertical()
            .width(Size::px(DECK_TILE_W))
            .child(FocusFrame::new(art, self.focused).scale(DECK_FOCUS_SCALE))
            .child(caption)
            .map(self.on_press.clone(), |el, handler| el.on_press(handler))
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
