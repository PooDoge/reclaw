use std::borrow::Cow;

use freya::prelude::*;

use crate::{metrics::*, prelude::*};

/// Library and catalog filter. A themed Freya `Input`; the explicit background is required or it
/// renders invisible on `bg_base`.
#[derive(Clone, PartialEq)]
pub struct SearchField {
    value: State<String>,
    placeholder: Cow<'static, str>,
    density: Density,
}

impl SearchField {
    pub fn new(value: State<String>) -> Self {
        Self {
            value,
            placeholder: Cow::Borrowed("Search library, tags, repos"),
            density: Density::Pointer,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<Cow<'static, str>>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }
}

impl Component for SearchField {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let colors = InputColorsThemePartial {
            background: Some(t.bg_raised.into()),
            focus_background: Some(t.bg_raised.into()),
            border_fill: Some(t.line_strong.into()),
            focus_border_fill: Some(t.accent.into()),
            color: Some(t.ink.into()),
            placeholder_color: Some(t.ink_subtle.into()),
        };
        let vertical = match self.density {
            Density::Pointer => SPACE_2,
            Density::Touch => SPACE_4,
        };
        let layout = InputLayoutThemePartial {
            corner_radius: Some(CornerRadius::new_all(RADIUS_MD).into()),
            padding: Some(Gaps::new(vertical, SPACE_3, vertical, SPACE_3).into()),
        };
        Input::new(self.value)
            .placeholder(self.placeholder.clone())
            .width(Size::fill())
            .leading(icon(IconName::Search, 16., t.ink_subtle))
            .theme_colors(colors)
            .theme_layout(layout)
    }
}
