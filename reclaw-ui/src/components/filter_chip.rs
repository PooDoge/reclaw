use std::borrow::Cow;

use freya::prelude::*;

use super::PressHandler;
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Tag and status filter (Quiver's tag filters). A themed Freya `Chip` with an optional mono count.
#[derive(Clone, PartialEq)]
pub struct FilterChip {
    label: Cow<'static, str>,
    count: Option<u32>,
    selected: bool,
    on_press: Option<PressHandler>,
    key: DiffKey,
}

impl KeyExt for FilterChip {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl FilterChip {
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self { label: label.into(), count: None, selected: false, on_press: None, key: DiffKey::None }
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_press(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_press = Some(handler.into());
        self
    }
}

impl Component for FilterChip {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let theme = ChipThemePartial {
            background: Some(CLEAR_FILL.into()),
            hover_background: Some(t.bg_raised.into()),
            selected_background: Some(t.accent.into()),
            border_fill: Some(t.line_strong.into()),
            selected_border_fill: Some(t.accent.into()),
            hover_border_fill: Some(t.line_strong.into()),
            focus_border_fill: Some(t.accent.into()),
            corner_radius: Some(CornerRadius::new_all(RADIUS_SM).into()),
            height: Some(Size::px(26.).into()),
            padding: Some(Gaps::new(0., SPACE_3, 0., SPACE_3).into()),
            color: Some(t.ink_muted.into()),
            hover_color: Some(t.ink.into()),
            selected_color: Some(t.on_accent.into()),
            selected_icon_fill: Some(t.on_accent.into()),
            hover_icon_fill: Some(t.ink.into()),
            ..Default::default()
        };
        let fg = if self.selected { t.on_accent } else { t.ink_muted };
        Chip::new().selected(self.selected).theme(theme).map(self.on_press.clone(), |el, handler| el.on_press(handler)).child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(SPACE_1)
                .child(TypeStyle::Label.text(self.label.clone(), fg).max_lines(1))
                // On one line whatever the row has left: a chip at the end of a wrapped row once broke "61" into "6" over "1".
                .maybe_child(self.count.map(|n| TypeStyle::Mono.text(n.to_string(), fg).max_lines(1))),
        )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

const CLEAR_FILL: Color = super::CLEAR;
