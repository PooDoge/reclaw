use std::borrow::Cow;

use freya::prelude::*;

use super::PressHandler;
use crate::{metrics::*, prelude::*, typography::TypeStyle};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonVariant {
    /// The one verb that changes a game's state: Install, Play, Update.
    Install,
    Primary,
    #[default]
    Secondary,
    Ghost,
    /// Only for a confirmed destructive step.
    Danger,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonSize {
    #[default]
    Md,
    /// The hero action.
    Lg,
    /// Touch and controller density, 48px.
    Touch,
}

impl ButtonSize {
    fn height(self) -> f32 {
        match self {
            Self::Md => 32.,
            Self::Lg => 44.,
            Self::Touch => 48.,
        }
    }

    fn icon(self) -> f32 {
        match self {
            Self::Md => 16.,
            _ => 18.,
        }
    }
}

/// Reclaw's call-to-action button: a themed Freya `Button` with an optional Lucide icon.
#[derive(Clone, PartialEq)]
pub struct ActionButton {
    variant: ButtonVariant,
    size: ButtonSize,
    icon: Option<IconName>,
    label: Option<Cow<'static, str>>,
    enabled: bool,
    on_press: Option<PressHandler>,
    key: DiffKey,
}

impl KeyExt for ActionButton {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl ActionButton {
    pub fn new(variant: ButtonVariant) -> Self {
        Self {
            variant,
            size: ButtonSize::Md,
            icon: None,
            label: None,
            enabled: true,
            on_press: None,
            key: DiffKey::None,
        }
    }

    pub fn install() -> Self {
        Self::new(ButtonVariant::Install)
    }

    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn on_press(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_press = Some(handler.into());
        self
    }
}

impl Component for ActionButton {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (bg, hover, border, fg) = match self.variant {
            ButtonVariant::Install => (t.install, t.install_hover, t.install, t.on_install),
            ButtonVariant::Primary => (t.accent, t.accent_hover, t.accent, t.on_accent),
            ButtonVariant::Secondary => (t.bg_raised, t.line, t.line_strong, t.ink),
            ButtonVariant::Ghost => (super::CLEAR, t.bg_raised, super::CLEAR, t.ink_muted),
            ButtonVariant::Danger => (t.danger_bg, t.danger_bg, t.danger, t.danger),
        };
        let height = self.size.height();
        let icon_only = self.label.is_none();
        let pad = if icon_only {
            0.
        } else if self.size == ButtonSize::Md {
            SPACE_4
        } else {
            SPACE_5
        };

        let colors = ButtonColorsThemePartial {
            background: Some(bg.into()),
            hover_background: Some(hover.into()),
            border_fill: Some(border.into()),
            focus_border_fill: Some(t.accent.into()),
            color: Some(fg.into()),
        };
        let layout = ButtonLayoutThemePartial {
            corner_radius: Some(CornerRadius::new_all(RADIUS_MD).into()),
            padding: Some(Gaps::new(0., pad, 0., pad).into()),
            height: Some(Size::px(height).into()),
            width: icon_only.then(|| Size::px(height).into()),
            ..Default::default()
        };

        let content = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(SPACE_2)
            .maybe_child(self.icon.map(|name| icon(name, self.size.icon(), fg)))
            .maybe_child(
                self.label
                    .clone()
                    .map(|text| TypeStyle::Label.text(text, fg)),
            );

        Button::new()
            .filled()
            .enabled(self.enabled)
            .theme_colors(colors)
            .theme_layout(layout)
            .map(self.on_press.clone(), |el, handler| el.on_press(handler))
            .child(content)
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
