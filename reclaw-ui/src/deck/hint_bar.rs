use freya::prelude::*;
use reclaw_input::{Action, ActionMap, ControllerKind};

use super::{LastInput, glyph::ButtonGlyph};
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

/// Contextual button prompts along the bottom safe zone. The glyph comes from the action map, so
/// a rebinding changes the bar; the set follows the last-used device.
#[derive(Clone, PartialEq)]
pub struct HintBar {
    hints: Vec<(Action, &'static str)>,
    kind: ControllerKind,
    last_input: LastInput,
    map: ActionMap,
}

impl HintBar {
    pub fn new(
        hints: Vec<(Action, &'static str)>,
        kind: ControllerKind,
        last_input: LastInput,
        map: ActionMap,
    ) -> Self {
        Self {
            hints,
            kind,
            last_input,
            map,
        }
    }
}

impl Component for HintBar {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let kind = match self.last_input {
            LastInput::Gamepad(k) => k,
            _ => self.kind,
        };
        let items = self.hints.iter().filter_map(|(action, text)| {
            let glyph = match self.last_input {
                LastInput::Gamepad(_) => ButtonGlyph::new(self.map.glyph(*action, kind)?),
                LastInput::Keyboard | LastInput::Pointer => ButtonGlyph::keycap(key_for(*action)?),
            };
            Some(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_2)
                    .child(glyph)
                    .child(TypeStyle::DeckHint.text(*text, t.ink_muted))
                    .into_element(),
            )
        });
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(SPACE_6)
            .width(Size::fill())
            .height(Size::px(DECK_HINT_H))
            .children(items)
    }
}
