use freya::prelude::*;
use reclaw_input::{Action, ActionMap, ControllerKind};

use super::glyph::ButtonGlyph;
use crate::deck::{LastInput, Section};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Section strip with the bumper glyphs at each end. Not a focus target: the bumpers move it.
#[derive(Clone, PartialEq)]
pub struct SectionTabs {
    current: Section,
    kind: ControllerKind,
    last_input: LastInput,
    map: ActionMap,
}

impl SectionTabs {
    pub fn new(current: Section, kind: ControllerKind, last_input: LastInput, map: ActionMap) -> Self {
        Self { current, kind, last_input, map }
    }

    fn glyph(&self, action: Action, keycap: &'static str) -> ButtonGlyph {
        match self.last_input {
            LastInput::Gamepad(k) => match self.map.glyph(action, k) {
                Some(face) => ButtonGlyph::new(face),
                None => ButtonGlyph::keycap(keycap),
            },
            _ => ButtonGlyph::keycap(keycap),
        }
    }
}

impl Component for SectionTabs {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let _ = self.kind;
        let tabs = Section::ALL.iter().map(|s| {
            let on = *s == self.current;
            rect()
                .vertical()
                .padding(Gaps::new(0., SPACE_2, 0., SPACE_2))
                .height(Size::px(DECK_TABS_H))
                .main_align(Alignment::Center)
                .border(
                    Border::new()
                        .fill(if on { t.accent } else { crate::components::CLEAR })
                        .width(BorderWidth { bottom: 4., ..Default::default() })
                        .alignment(BorderAlignment::Inner),
                )
                .child(TypeStyle::DeckHeading.text(s.label(), if on { t.ink } else { t.ink_muted }))
                .into_element()
        });
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(SPACE_5)
            .width(Size::fill())
            .height(Size::px(DECK_TABS_H))
            .child(self.glyph(Action::PrevSection, "["))
            .children(tabs)
            .child(self.glyph(Action::NextSection, "]"))
    }
}
