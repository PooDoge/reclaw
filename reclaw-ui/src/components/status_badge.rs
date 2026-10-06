use std::borrow::Cow;

use freya::prelude::*;

use super::{FailedHint, PressHandler};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Foreground and fill for a status. Shared by badges and the library row dot.
pub fn status_tone(t: &Reclaw, status: AppStatus) -> (Color, Color) {
    match status {
        AppStatus::Installed => (t.ok, t.ok_bg),
        AppStatus::UpdateReady => (t.warn, t.warn_bg),
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
        AppStatus::Available => None,
    }
}

/// One badge per game state. Always a word, plus an icon except for "Not installed", so state
/// never depends on hue alone. A Failed badge given a [`failure`](Self::failure) text shows it on
/// hover and, given [`on_failure`](Self::on_failure), opens the job's log when pressed.
#[derive(Clone, PartialEq)]
pub struct StatusBadge {
    status: AppStatus,
    running: bool,
    label: Option<Cow<'static, str>>,
    failure: Option<String>,
    on_failure: Option<PressHandler>,
}

impl StatusBadge {
    pub fn new(status: AppStatus) -> Self {
        Self { status, running: false, label: None, failure: None, on_failure: None }
    }

    /// The app is running right now. Takes priority over its install state on tiles and heroes.
    pub fn running() -> Self {
        Self { status: AppStatus::Installed, running: true, label: None, failure: None, on_failure: None }
    }

    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Why it failed, in one short line, for the tooltip. Only a Failed badge uses it.
    pub fn failure(mut self, text: impl Into<String>) -> Self {
        self.failure = Some(text.into());
        self
    }

    /// Pressing a Failed badge: open the job's report and log.
    pub fn on_failure(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_failure = Some(handler.into());
        self
    }
}

impl Component for StatusBadge {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (fg, bg) = status_tone(&t, self.status);
        let (icon_name, default_text) =
            if self.running { (Some(IconName::Play), "Running") } else { (status_icon(self.status), self.status.label()) };
        let text = self.label.clone().unwrap_or(Cow::Borrowed(default_text));

        let badge = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(6.)
            .height(Size::px(20.))
            .padding(Gaps::new(0., SPACE_2, 0., SPACE_2))
            .corner_radius(RADIUS_SM)
            .background(bg)
            .maybe_child(icon_name.map(|name| icon(name, 12., fg)))
            .child(TypeStyle::Eyebrow.text(text, fg));
        match (&self.failure, self.status == AppStatus::Failed && !self.running) {
            (Some(why), true) => {
                FailedHint::new(badge, why.clone()).map(self.on_failure.clone(), |hint, h| hint.on_press(h)).into_element()
            }
            _ => badge.into_element(),
        }
    }
}
