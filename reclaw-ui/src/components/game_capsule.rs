use freya::prelude::*;

use super::{ArtPlaceholder, PressHandler, RemoteArt, StatusBadge, hoverable, pointer_cursor};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Steam-style library tile: 3:4 art, title, project, status badge. Hover draws an accent border
/// and, for installed games, a Play button over the art. Touch has no hover, so Play lives in the
/// hero there.
#[derive(Clone, PartialEq)]
pub struct GameCapsule {
    game: GameEntry,
    selected: bool,
    fluid: bool,
    hovered: bool,
    on_press: Option<PressHandler>,
    on_play: Option<PressHandler>,
    key: DiffKey,
}

impl KeyExt for GameCapsule {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl GameCapsule {
    pub fn new(game: GameEntry) -> Self {
        Self { game, selected: false, fluid: false, hovered: false, on_press: None, on_play: None, key: DiffKey::None }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Fill the parent's width (compact and phone grids) instead of the fixed `CAPSULE_W`.
    pub fn fluid(mut self, fluid: bool) -> Self {
        self.fluid = fluid;
        self
    }

    /// Render in the hover state regardless of the pointer: for galleries and snapshots.
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn on_press(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_press = Some(handler.into());
        self
    }

    pub fn on_play(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_play = Some(handler.into());
        self
    }
}

impl Component for GameCapsule {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let width = if self.fluid { Size::fill() } else { Size::px(CAPSULE_W) };
        let hot = hovering() || self.hovered;
        let lit = self.selected || hot;
        let border = Border::new()
            .fill(if lit { t.accent } else { t.line })
            .width(if self.selected { 2. } else { 1. })
            .alignment(BorderAlignment::Inner);
        let show_play = hot && self.game.status.is_installed();

        let art = rect()
            .width(Size::fill())
            .child(RemoteArt::new(self.game.art.capsule.clone(), ArtPlaceholder::capsule(Size::fill()), Size::fill(), Size::px(CAPSULE_H)))
            .child(
                rect()
                    .position(Position::new_absolute().top(SPACE_2).left(SPACE_2))
                    .child(SystemBadge::new(self.game.platform).over_art(true)),
            )
            .maybe(show_play, |el| {
                el.child(
                    rect()
                        .position(Position::new_absolute().top(CAPSULE_H - 44. - SPACE_2).right(SPACE_2))
                        .child(ActionButton::install().icon(IconName::Play).label("Play").map(self.on_play.clone(), |b, h| b.on_press(h))),
                )
            });

        let body = rect()
            .vertical()
            .width(Size::fill())
            .spacing(6.)
            .padding(Gaps::new(SPACE_2, SPACE_3, SPACE_3, SPACE_3))
            .child(TypeStyle::Label.text(self.game.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis))
            .child(TypeStyle::Meta.text(self.game.project.clone(), t.ink_subtle).max_lines(1).text_overflow(TextOverflow::Ellipsis))
            .child(StatusBadge::new(self.game.status));

        let card = rect()
            .vertical()
            .width(width)
            .background(t.bg_panel)
            .border(border)
            .corner_radius(RADIUS_MD)
            .overflow(Overflow::Clip)
            .maybe(lit, |el| el.shadow(Shadow::new().y(2.).blur(8.).color(Color::from_argb(90, 0, 0, 0))))
            .child(art)
            .child(body)
            .map(self.on_press.clone(), |el, handler| el.on_press(handler));

        pointer_cursor(hoverable(card, hovering))
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
