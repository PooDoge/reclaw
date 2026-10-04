use freya::prelude::*;
use reclaw_input::{Action, ActionMap, ControllerKind};

use super::glyph::ButtonGlyph;
use crate::deck::LastInput;
use crate::{metrics::*, prelude::*, typography::TypeStyle};

fn key_for(action: Action) -> Option<&'static str> {
    match action {
        Action::Confirm => Some("Enter"),
        Action::Back => Some("Esc"),
        Action::MainMenu => Some("Tab"),
        Action::QuickAccess => Some("Shift+Tab"),
        Action::PrevSection => Some("["),
        Action::NextSection => Some("]"),
        // No default keyboard key (see contract input.keyboardFallback): hidden in keyboard mode.
        _ => None,
    }
}

/// Hints that open a menu sit at the left edge and the rest (Options, Select, Back) at the right,
/// as in Big Picture. Everything else is "contextual" and goes right.
fn is_leading(action: Action) -> bool {
    matches!(action, Action::MainMenu | Action::QuickAccess)
}

/// Contextual button prompts along the bottom safe zone. The glyph comes from the action map, so
/// a rebinding changes the bar; the set follows the last-used device. `tight` shrinks the gaps for
/// windows too narrow to fit them at full size.
#[derive(Clone, PartialEq)]
pub struct HintBar {
    hints: Vec<(Action, &'static str)>,
    kind: ControllerKind,
    last_input: LastInput,
    map: ActionMap,
    tight: bool,
}

impl HintBar {
    pub fn new(hints: Vec<(Action, &'static str)>, kind: ControllerKind, last_input: LastInput, map: ActionMap) -> Self {
        Self { hints, kind, last_input, map, tight: false }
    }

    pub fn tight(mut self, tight: bool) -> Self {
        self.tight = tight;
        self
    }
}

impl Component for HintBar {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let kind = match self.last_input {
            LastInput::Gamepad(k) => k,
            _ => self.kind,
        };
        let gap = if self.tight { SPACE_3 } else { SPACE_6 };
        let item = |action: Action, text: &'static str| {
            let glyph = match self.last_input {
                LastInput::Gamepad(_) => ButtonGlyph::new(self.map.glyph(action, kind)?),
                LastInput::Keyboard | LastInput::Pointer => ButtonGlyph::keycap(key_for(action)?),
            };
            Some(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_2)
                    .child(glyph)
                    .child(TypeStyle::DeckHint.text(text, t.ink_muted).max_lines(1))
                    .into_element(),
            )
        };
        let group = |leading: bool| {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(gap)
                .children(self.hints.iter().filter(move |(a, _)| is_leading(*a) == leading).filter_map(|(a, text)| item(*a, text)))
        };
        rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .height(Size::px(DECK_HINT_H))
            .child(group(true))
            .child(rect().width(Size::flex(1.)))
            .child(group(false))
    }
}
