use freya::prelude::*;

use crate::prelude::*;

/// Full-bleed "Big Art" layer behind the page. Text never sits on the art itself: the art is
/// blurred and covered by `deck-scrim`, so ink and ink-muted keep 4.5:1 on any image.
///
/// With no art (placeholder catalog) it shows `deck-bg` with a wash of `tint` fading to nothing,
/// so the focused game still changes the room's color.
#[derive(Clone, PartialEq)]
pub struct Backdrop {
    tint: Color,
    art: Option<ImageSource>,
}

impl Backdrop {
    pub fn new(tint: Color) -> Self {
        Self { tint, art: None }
    }

    pub fn art(mut self, art: impl Into<ImageSource>) -> Self {
        self.art = Some(art.into());
        self
    }
}

impl Component for Backdrop {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let wash = Color::from_argb(72, self.tint.r(), self.tint.g(), self.tint.b());
        let none = Color::from_argb(0, self.tint.r(), self.tint.g(), self.tint.b());

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(t.deck_bg)
            .maybe_child(
                self.art
                    .clone()
                    .map(|src| ImageViewer::new(src).width(Size::fill()).height(Size::fill()).image_cover(ImageCover::Center).blur(24.)),
            )
            .child(
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(t.deck_scrim),
            )
            .child(
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(LinearGradient::new().angle(180.).stop((wash, 0.)).stop((none, 70.))),
            )
    }
}
