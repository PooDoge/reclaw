//! The type scale from tokens.json as one enum, so no call site hand-picks a size or weight.
use std::borrow::Cow;

use freya::prelude::*;

pub const FONT_SANS: &str = "Hanken Grotesk";
pub const FONT_MONO: &str = "JetBrains Mono";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TypeStyle {
    TitleHero,
    TitlePage,
    Heading,
    Body,
    BodyTouch,
    Label,
    Meta,
    /// Uppercase section labels and top-nav items; the text is uppercased here.
    Eyebrow,
    Mono,
    DeckTitle,
    DeckHeading,
    DeckBody,
    DeckLabel,
    DeckMeta,
    DeckHint,
}

impl TypeStyle {
    pub(crate) fn spec(self) -> (f32, FontWeight, &'static str) {
        match self {
            Self::TitleHero => (32., FontWeight::BOLD, FONT_SANS),
            Self::TitlePage => (22., FontWeight::BOLD, FONT_SANS),
            Self::Heading => (16., FontWeight::SEMI_BOLD, FONT_SANS),
            Self::Body => (14., FontWeight::NORMAL, FONT_SANS),
            Self::BodyTouch => (16., FontWeight::NORMAL, FONT_SANS),
            Self::Label => (13., FontWeight::SEMI_BOLD, FONT_SANS),
            Self::Meta => (12., FontWeight::NORMAL, FONT_SANS),
            Self::Eyebrow => (11., FontWeight::BOLD, FONT_SANS),
            Self::Mono => (12., FontWeight::NORMAL, FONT_MONO),
            Self::DeckTitle => (40., FontWeight::BOLD, FONT_SANS),
            Self::DeckHeading => (24., FontWeight::SEMI_BOLD, FONT_SANS),
            Self::DeckBody => (20., FontWeight::NORMAL, FONT_SANS),
            Self::DeckLabel => (18., FontWeight::SEMI_BOLD, FONT_SANS),
            Self::DeckMeta => (16., FontWeight::NORMAL, FONT_SANS),
            Self::DeckHint => (16., FontWeight::SEMI_BOLD, FONT_SANS),
        }
    }

    /// A single-line label in this style. `color` is always explicit: the theme-ext helpers read
    /// Freya's palette, not Reclaw's.
    pub fn text(self, text: impl Into<Cow<'static, str>>, color: Color) -> Label {
        let (size, weight, family) = self.spec();
        let text = text.into();
        let text: Cow<'static, str> = if self == Self::Eyebrow { Cow::Owned(text.to_uppercase()) } else { text };
        label().text(text).font_size(size).font_weight(weight).font_family(family).color(color)
    }
}
