use freya::prelude::*;

use super::Slot;
use crate::{
    activity::{ActivityId, LineTone, line_tone, report},
    desktop::use_desktop_ui,
    effect::Effect,
    metrics::*,
    prelude::*,
    shell::use_shell,
    store::use_activity,
    surface::{Dialog, DialogAction, SurfaceKind},
    typography::TypeStyle,
};

/// The tallest the log box grows before it scrolls; the dialog's own text and buttons stay in view.
const LOG_HEIGHT: f32 = 320.;
/// One mono line and the gap after it, to size a short log's box to its lines.
const LINE_HEIGHT: f32 = 18.;

/// "X failed": the whole reason, what to do about it, and the lines the job wrote to the log, problems in color. Opened by
/// pressing a "Failed" label or badge. A popup on desktop, a full-screen page on touch.
#[derive(Clone, PartialEq)]
pub(super) struct FailureLogView {
    pub open: Slot<ActivityId>,
}

impl Component for FailureLogView {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui) = (use_shell(), use_desktop_ui());
        let env = *ui.env.read();
        let board = use_activity();
        let open = self.open;
        let Some(id) = open.get() else { return rect().into_element() };
        // Dismissed from Downloads while open: nothing left to show.
        let Some(report) = report(&board, id) else { return rect().into_element() };

        let close = EventHandler::new(move |()| open.close());
        let on_effect = shell.on_effect.clone();
        let folder = EventHandler::new(move |()| on_effect.call(Effect::OpenLogFolder));

        let tone = |line: &str| match line_tone(line) {
            LineTone::Error => t.danger,
            LineTone::Warning => t.warn,
            LineTone::Plain => t.ink_muted,
        };
        let log: Element = if report.log.is_empty() {
            rect()
                .padding(SPACE_3)
                .child(TypeStyle::Meta.text("Nothing was recorded for this job. The log file may have more.", t.ink_subtle))
                .into_element()
        } else {
            let height = (report.log.len() as f32 * LINE_HEIGHT + 2. * SPACE_3).min(LOG_HEIGHT);
            ScrollView::new()
                .width(Size::fill())
                .height(Size::px(height))
                .child(
                    rect()
                        .vertical()
                        .spacing(2.)
                        .width(Size::fill())
                        .padding(SPACE_3)
                        .children(report.log.iter().map(|line| TypeStyle::Mono.text(line.clone(), tone(line)).into_element())),
                )
                .into_element()
        };

        let body = rect()
            .vertical()
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(TypeStyle::Body.text(report.reason.clone(), t.ink))
            .children(report.details.iter().map(|d| TypeStyle::Meta.text(d.clone(), t.ink_muted).into_element()))
            .child(TypeStyle::Eyebrow.text("Log", t.ink_subtle))
            .child(
                rect()
                    .width(Size::fill())
                    .background(t.bg_deep)
                    .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
                    .corner_radius(RADIUS_MD)
                    .child(log),
            );
        let actions = vec![
            DialogAction::new("Open log folder", ButtonVariant::Ghost, folder).icon(IconName::Folder),
            DialogAction::new("Close", ButtonVariant::Secondary, close.clone()),
        ];
        Dialog::new(SurfaceKind::Form, env.surface(), env.window, format!("{} failed", report.title), body, actions, close)
            .wide(true)
            .into_element()
    }
}
