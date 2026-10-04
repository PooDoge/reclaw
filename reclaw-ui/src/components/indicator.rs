use freya::prelude::*;

use super::CLEAR;
use crate::{
    activity::{Indicator, IndicatorKind},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

/// The icon and color that mean each kind of background work, the same on a desktop row and a Deck
/// card. Color never carries the meaning alone: the icon and the label do too.
pub fn indicator_look(t: &Reclaw, kind: IndicatorKind) -> (IconName, Color) {
    match kind {
        IndicatorKind::UpdateAvailable => (IconName::Download, t.warn),
        IndicatorKind::Queued => (IconName::Clock, t.ink_muted),
        IndicatorKind::Downloading => (IconName::Queue, t.accent),
        IndicatorKind::Installing => (IconName::Package, t.info),
        IndicatorKind::Done => (IconName::Check, t.ok),
        IndicatorKind::Failed => (IconName::Alert, t.danger),
        IndicatorKind::Mods => (IconName::Mods, t.accent),
    }
}

/// A thin progress bar, drawn as a track and a fill so it can sit on an edge of a card.
pub fn progress_strip(t: &Reclaw, kind: IndicatorKind, progress: Option<f32>, height: f32) -> Rect {
    let (_, color) = indicator_look(t, kind);
    let fraction = progress.unwrap_or(0.).clamp(0., 1.);
    rect()
        .width(Size::fill())
        .height(Size::px(height))
        .background(t.bg_raised)
        .child(rect().width(Size::percent(fraction * 100.)).height(Size::fill()).background(color))
}

/// The icon and short label of a game's background work: "34%", "Building", "Updated", "Update ready".
#[derive(Clone, PartialEq)]
pub struct IndicatorBadge {
    indicator: Indicator,
    /// Deck cards use the larger type and a filled chip so the badge reads from across a room.
    large: bool,
    filled: bool,
}

impl IndicatorBadge {
    pub fn new(indicator: Indicator) -> Self {
        Self { indicator, large: false, filled: false }
    }

    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// Draw it as a chip on a solid fill (for use over art) rather than bare.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }
}

impl Component for IndicatorBadge {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (icon_name, color) = indicator_look(&t, self.indicator.kind);
        let size = if self.large { 22. } else { 14. };
        let style = if self.large { TypeStyle::DeckMeta } else { TypeStyle::Meta };
        let mods = (self.indicator.mods > 0 && self.indicator.kind != IndicatorKind::Mods).then(|| format!("+{}", self.indicator.mods));
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(if self.large { SPACE_2 } else { 6. })
            .padding(if self.filled { Gaps::new(4., 8., 4., 8.) } else { Gaps::new(0., 0., 0., 0.) })
            .corner_radius(RADIUS_MD)
            .background(if self.filled { t.bg_base } else { CLEAR })
            .child(icon(icon_name, size, color))
            .child(style.text(self.indicator.label.clone(), if self.filled { t.ink } else { color }))
            .maybe_child(mods.map(|m| TypeStyle::Mono.text(m, t.ink_muted)))
    }
}
