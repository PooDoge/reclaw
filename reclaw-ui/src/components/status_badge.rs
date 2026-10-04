use std::borrow::Cow;

use freya::prelude::*;

use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Foreground and fill for a status. Shared by badges and the library row dot.
pub fn status_tone(t: &Reclaw, status: AppStatus) -> (Color, Color) {
    match status {
        AppStatus::Installed => (t.ok, t.ok_bg),
        AppStatus::UpdateReady | AppStatus::NeedsFile => (t.warn, t.warn_bg),
        AppStatus::Installing => (t.info, t.info_bg),
        AppStatus::Failed => (t.danger, t.danger_bg),
        AppStatus::Available => (t.ink_muted, t.bg_raised),
    }
}

fn status_icon(status: AppStatus) -> Option<IconName> {
    match status {
        AppStatus::Installed => Some(IconName::Check),
        AppStatus::UpdateReady => Some(IconName::Download),
        AppStatus::Installing => Some(IconName::Queue),
        AppStatus::Failed => Some(IconName::Alert),
        AppStatus::NeedsFile => Some(IconName::File),
        AppStatus::Available => None,
    }
}

/// One badge per game state. Always a word, plus an icon except for "Not installed", so state
/// never depends on hue alone.
#[derive(Clone, PartialEq)]
pub struct StatusBadge {
    status: AppStatus,
    label: Option<Cow<'static, str>>,
}

impl StatusBadge {
    pub fn new(status: AppStatus) -> Self {
        Self {
            status,
            label: None,
        }
    }

    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Component for StatusBadge {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (fg, bg) = status_tone(&t, self.status);
        let text = self
            .label
            .clone()
            .unwrap_or(Cow::Borrowed(self.status.label()));

        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(6.)
            .height(Size::px(20.))
            .padding(Gaps::new(0., SPACE_2, 0., SPACE_2))
            .corner_radius(RADIUS_SM)
            .background(bg)
            .maybe_child(status_icon(self.status).map(|name| icon(name, 12., fg)))
            .child(TypeStyle::Eyebrow.text(text, fg))
    }
}
