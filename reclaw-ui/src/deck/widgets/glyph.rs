use std::borrow::Cow;

use freya::prelude::*;
use reclaw_input::GlyphFace;

use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// A controller button glyph, or a keyboard keycap. Neutral on purpose: a raised circle with a
/// ring and an ink letter, no vendor colors, so contrast is the same for every controller.
#[derive(Clone, PartialEq)]
pub struct ButtonGlyph {
    face: GlyphFace,
    keycap: Option<Cow<'static, str>>,
}

impl ButtonGlyph {
    pub fn new(face: GlyphFace) -> Self {
        Self { face, keycap: None }
    }

    pub fn keycap(label: impl Into<Cow<'static, str>>) -> Self {
        Self { face: GlyphFace::Label(""), keycap: Some(label.into()) }
    }
}

impl Component for ButtonGlyph {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let shape = |name| icon(name, 16., t.ink).into_element();
        let (text, radius): (Option<String>, f32) = match (&self.keycap, self.face) {
            (Some(label), _) => (Some(label.to_string()), RADIUS_MD),
            (None, GlyphFace::Label(s)) => (Some(s.to_string()), 14.),
            (None, _) => (None, 14.),
        };
        let content = match (text, self.face) {
            (Some(text), _) => TypeStyle::DeckHint.text(text, t.ink).into_element(),
            (None, GlyphFace::Cross) => shape(IconName::X),
            (None, GlyphFace::Circle) => shape(IconName::Circle),
            (None, GlyphFace::Square) => shape(IconName::Square),
            (None, GlyphFace::Triangle) => shape(IconName::Triangle),
            (None, GlyphFace::Label(_)) => rect().into_element(),
        };
        rect()
            .center()
            .min_width(Size::px(28.))
            .height(Size::px(28.))
            .padding(Gaps::new(0., 8., 0., 8.))
            .corner_radius(radius)
            .background(t.bg_raised)
            .border(Border::new().fill(t.line_strong).width(2.).alignment(BorderAlignment::Inner))
            .child(content)
    }
}
