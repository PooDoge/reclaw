use freya::prelude::*;

use super::{ArtPlaceholder, PressHandler};
use crate::{
    activity::{Activity, Outcome, format_bytes, format_eta, format_rate},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

/// Download queue row for one activity. The stage label names the pipeline step. The fill is accent
/// while working, ok once finished, and danger on failure, where the reason replaces the detail
/// line so a red bar never appears without a cause. The button cancels a running job and removes
/// an ended one.
#[derive(Clone, PartialEq)]
pub struct DownloadItem {
    activity: Activity,
    on_cancel: Option<PressHandler>,
    controller: bool,
    cancel_focused: bool,
    key: DiffKey,
}

impl KeyExt for DownloadItem {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl DownloadItem {
    pub fn new(activity: Activity) -> Self {
        Self { activity, on_cancel: None, controller: false, cancel_focused: false, key: DiffKey::None }
    }

    /// Deck mode: larger text and padding, and the button shows the focus frame.
    pub fn controller(mut self, controller: bool, cancel_focused: bool) -> Self {
        self.controller = controller;
        self.cancel_focused = cancel_focused;
        self
    }

    pub fn on_cancel(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_cancel = Some(handler.into());
        self
    }
}

/// "21 MB of 61 MB", or just what has arrived when the size is not known.
pub fn transfer_line(a: &Activity) -> String {
    match a.bytes_total {
        Some(total) => format!("{} of {}", format_bytes(a.bytes_done), format_bytes(total)),
        None if a.bytes_done > 0 => format_bytes(a.bytes_done),
        None => String::new(),
    }
}

impl Component for DownloadItem {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let a = &self.activity;
        let failure = match &a.outcome {
            Outcome::Failed { reason } => Some(reason.clone()),
            _ => None,
        };
        let finished = a.outcome == Outcome::Finished;
        let fill = if failure.is_some() {
            t.danger
        } else if finished {
            t.ok
        } else {
            t.accent
        };
        let stage = if failure.is_some() {
            "Failed"
        } else if finished {
            "Done"
        } else {
            a.stage.label()
        };
        let detail = failure.clone().unwrap_or_else(|| transfer_line(a));
        let detail_color = if failure.is_some() { t.danger } else { t.ink_subtle };
        let speed = a.rate.filter(|r| *r > 0 && a.is_running()).map(format_rate);
        let eta = a.eta().map(format_eta);

        let percent = a.progress().map_or(0., |p| p * 100.);
        let bar =
            ProgressBar::new(percent.clamp(0., 100.)).show_progress(false).background(t.bg_raised).progress_background(fill).height(6.);

        let text_column = rect()
            .vertical()
            .spacing(6.)
            .width(Size::flex(1.))
            .child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(SPACE_3)
                    .width(Size::fill())
                    .child(
                        rect().width(Size::flex(1.)).child(
                            (if self.controller { TypeStyle::DeckLabel } else { TypeStyle::Label })
                                .text(a.title.clone(), t.ink)
                                .max_lines(1)
                                .text_overflow(TextOverflow::Ellipsis),
                        ),
                    )
                    .child(TypeStyle::Mono.text(stage, if failure.is_some() { t.danger } else { t.ink_muted })),
            )
            .child(bar)
            .child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(SPACE_3)
                    .width(Size::fill())
                    .child(rect().width(Size::flex(1.)).child(TypeStyle::Meta.text(detail, detail_color)))
                    .maybe_child(eta.map(|e| TypeStyle::Mono.text(e, t.ink_muted)))
                    .maybe_child(speed.map(|s| TypeStyle::Mono.text(s, t.ink_muted))),
            );

        rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_4)
            .width(Size::fill())
            .padding(if self.controller {
                Gaps::new(SPACE_4, SPACE_5, SPACE_4, SPACE_5)
            } else {
                Gaps::new(SPACE_3, SPACE_4, SPACE_3, SPACE_4)
            })
            .background(t.bg_panel)
            .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
            .corner_radius(RADIUS_MD)
            .child(ArtPlaceholder::thumb(92., 43.))
            .child(text_column)
            .child(crate::deck::FocusFrame::new(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::X)
                    .size(if self.controller { ButtonSize::Controller } else { ButtonSize::Md })
                    .map(self.on_cancel.clone(), |b, h| b.on_press(h)),
                self.cancel_focused,
            ))
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
