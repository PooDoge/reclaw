use freya::prelude::*;
use reclaw_games::project::Platform;

use super::CLEAR;
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// The mark of the system a game was recompiled from ("N64", "PS2"), as a small chip. Neutral in
/// color on purpose, like the controller glyphs: the letters carry the meaning, so it reads the same
/// on every theme and for everyone. Draws nothing for a game of no known system.
#[derive(Clone, PartialEq)]
pub struct SystemBadge {
    platform: Platform,
    /// Over artwork, on a solid fill so it stays legible whatever the picture is.
    over_art: bool,
    large: bool,
    key: DiffKey,
}

impl KeyExt for SystemBadge {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl SystemBadge {
    pub fn new(platform: Platform) -> Self {
        Self { platform, over_art: false, large: false, key: DiffKey::None }
    }

    pub fn over_art(mut self, over_art: bool) -> Self {
        self.over_art = over_art;
        self
    }

    /// The larger type Deck mode reads from across a room.
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }
}

impl Component for SystemBadge {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        if !self.platform.is_known() {
            return rect().into_element();
        }
        let style = if self.large { TypeStyle::DeckHint } else { TypeStyle::Mono };
        let pad = if self.large { Gaps::new(3., 8., 3., 8.) } else { Gaps::new(1., 6., 1., 6.) };
        rect()
            .padding(pad)
            .corner_radius(RADIUS_SM)
            .background(if self.over_art { t.bg_base } else { CLEAR })
            .border(Border::new().fill(t.line_strong).width(1.).alignment(BorderAlignment::Inner))
            .child(style.text(self.platform.short(), if self.over_art { t.ink } else { t.ink_muted }))
            .into_element()
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
