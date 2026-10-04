use freya::prelude::*;

use crate::prelude::*;

/// A themed Freya `Switch`. Always pair it with a text label to its left.
#[derive(Clone, PartialEq)]
pub struct ToggleSwitch {
    on: State<bool>,
    enabled: bool,
}

impl ToggleSwitch {
    pub fn new(on: State<bool>) -> Self {
        Self { on, enabled: true }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for ToggleSwitch {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let mut on = self.on;
        let colors = SwitchColorsThemePartial {
            background: Some(t.bg_raised.into()),
            thumb_background: Some(t.ink_muted.into()),
            toggled_background: Some(t.accent.into()),
            toggled_thumb_background: Some(t.on_accent.into()),
            focus_border_fill: Some(t.accent.into()),
        };
        Switch::new()
            .toggled(on)
            .enabled(self.enabled)
            .theme_colors(colors)
            .on_toggle(move |_| on.toggle())
    }
}
