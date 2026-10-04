use std::time::Duration;

use freya::{animation::*, prelude::*};

use crate::{
    activity::{Indicator, IndicatorKind},
    components::{IndicatorBadge, indicator_look, progress_strip},
    metrics::*,
    prelude::*,
};

/// Height of the bar along the bottom of a card's art.
pub const CARD_STRIP_H: f32 = 8.;

/// A segment that slides along a track, for work whose size is not known yet. A component of its own
/// so only the cards that need it run an animation.
#[derive(Clone, PartialEq)]
struct SlidingStrip {
    color: Color,
}

impl Component for SlidingStrip {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let animation = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            conf.on_finish(OnFinish::restart());
            AnimNum::new(0., 1.).duration(Duration::from_millis(1400)).function(Function::Quad).ease(Ease::InOut)
        });
        let p = animation.get().value();
        // The segment is a third of the track and travels from just off the left edge to just off the right.
        let segment = DECK_TILE_W / 3.;
        let x = -segment + p * (DECK_TILE_W + segment);
        rect()
            .width(Size::fill())
            .height(Size::px(CARD_STRIP_H))
            .background(t.bg_raised)
            .overflow(Overflow::Clip)
            .child(rect().width(Size::px(segment)).height(Size::fill()).offset_x(x).background(self.color))
    }
}

/// What a card shows over its art while its game has background work: a chip at the top left with the
/// icon and "34%" or "Building", and a bar along the bottom edge. The bar fills when the size is known,
/// slides when it is not, is full and green when finished, and absent when only an update is waiting.
///
/// Both sit inside the art's clip, so they scale with the focused card and keep its rounded corners.
#[derive(Clone, PartialEq)]
pub struct CardIndicator {
    indicator: Indicator,
}

impl CardIndicator {
    pub fn new(indicator: Indicator) -> Self {
        Self { indicator }
    }
}

impl Component for CardIndicator {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let i = &self.indicator;
        let (_, color) = indicator_look(&t, i.kind);
        let bar: Option<Element> = match (i.kind, i.progress) {
            (IndicatorKind::Done, _) => Some(progress_strip(&t, i.kind, Some(1.), CARD_STRIP_H).into_element()),
            (IndicatorKind::Failed | IndicatorKind::UpdateAvailable, _) => None,
            (_, Some(p)) => Some(progress_strip(&t, i.kind, Some(p), CARD_STRIP_H).into_element()),
            (_, None) if i.is_active() => Some(SlidingStrip { color }.into_element()),
            (_, None) => None,
        };
        rect()
            .position(Position::new_absolute().top(0.).left(0.))
            .width(Size::px(DECK_TILE_W))
            .height(Size::px(DECK_TILE_H))
            .interactive(Interactive::No)
            .child(
                rect()
                    .position(Position::new_absolute().top(SPACE_3).left(SPACE_3))
                    .child(IndicatorBadge::new(i.clone()).large(true).filled(true)),
            )
            // `bottom()` does not anchor like CSS here: place the bar by its top edge.
            .maybe_child(bar.map(|bar| {
                rect().position(Position::new_absolute().top(DECK_TILE_H - CARD_STRIP_H).left(0.)).width(Size::px(DECK_TILE_W)).child(bar)
            }))
    }
}
