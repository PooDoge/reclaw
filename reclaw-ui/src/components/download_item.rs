use freya::prelude::*;

use super::{ArtPlaceholder, PressHandler};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Download queue row. The stage label names the recompilation pipeline step. The fill is accent
/// while working, ok once fully verified, and danger on failure, where the reason replaces the
/// detail line so a red bar never appears without a cause.
#[derive(Clone, PartialEq)]
pub struct DownloadItem {
    download: Download,
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
    pub fn new(download: Download) -> Self {
        Self { download, on_cancel: None, controller: false, cancel_focused: false, key: DiffKey::None }
    }

    /// Deck mode: larger text and padding, and the Cancel button shows the focus frame.
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

impl Component for DownloadItem {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let d = &self.download;
        let failed = d.error.is_some();
        let fill = if failed {
            t.danger
        } else if d.progress >= 100. {
            t.ok
        } else {
            t.accent
        };
        let stage = if failed { "Failed" } else { d.stage.label() };
        let detail = d.error.clone().unwrap_or_else(|| d.detail.clone());
        let detail_color = if failed { t.danger } else { t.ink_subtle };

        let bar =
            ProgressBar::new(d.progress.clamp(0., 100.)).show_progress(false).background(t.bg_raised).progress_background(fill).height(6.);

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
                                .text(d.title.clone(), t.ink)
                                .max_lines(1)
                                .text_overflow(TextOverflow::Ellipsis),
                        ),
                    )
                    .child(TypeStyle::Mono.text(stage, if failed { t.danger } else { t.ink_muted })),
            )
            .child(bar)
            .child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(SPACE_3)
                    .width(Size::fill())
                    .child(rect().width(Size::flex(1.)).child(TypeStyle::Meta.text(detail, detail_color)))
                    .maybe_child(d.speed.clone().map(|s| TypeStyle::Mono.text(s, t.ink_muted))),
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
