use freya::prelude::*;

use super::{PressHandler, pointer_cursor};

/// Wraps a "Failed" indicator (a Downloads row's stage label, a status badge): hovering shows the short reason in a tooltip, and
/// pressing, when a handler is given, opens the full report with the job's log. Without a handler it is the tooltip alone (a
/// capsule, whose own press opens the game).
#[derive(Clone, PartialEq)]
pub struct FailedHint {
    child: Element,
    text: String,
    on_press: Option<PressHandler>,
    position: AttachedPosition,
    key: DiffKey,
}

impl KeyExt for FailedHint {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl FailedHint {
    /// `text` is one line: the reason already shortened (`activity::short_reason`).
    pub fn new(child: impl IntoElement, text: impl Into<String>) -> Self {
        Self { child: child.into_element(), text: text.into(), on_press: None, position: AttachedPosition::Bottom, key: DiffKey::None }
    }

    pub fn on_press(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_press = Some(handler.into());
        self
    }

    /// Where the tooltip opens; below by default. A label at the right edge of a row opens it to the left, so it is not cut off.
    pub fn position(mut self, position: AttachedPosition) -> Self {
        self.position = position;
        self
    }
}

impl Component for FailedHint {
    fn render(&self) -> impl IntoElement {
        let pressable = self.on_press.is_some();
        let target = rect()
            .a11y_role(if pressable { AccessibilityRole::Button } else { AccessibilityRole::Label })
            .a11y_alt(if pressable { format!("Failed: {}. Show the log", self.text) } else { format!("Failed: {}", self.text) })
            .child(self.child.clone())
            .map(self.on_press.clone(), |el, handler| el.on_press(handler));
        let target = if pressable { pointer_cursor(target) } else { target };
        TooltipContainer::new(Tooltip::new_text(self.text.clone())).position(self.position).child(target)
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
