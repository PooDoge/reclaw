use freya::prelude::*;

use super::common::{empty_state, page_scroll};
use crate::{
    desktop::use_desktop_ui, effect::Effect, metrics::*, prelude::*, shell::use_shell, store::use_activity, typography::TypeStyle,
};

/// The download queue: progress, errors (hover "Failed" for why, press it for the log), and a way to cancel.
#[derive(PartialEq)]
pub struct DownloadsPage {}

impl Component for DownloadsPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui) = (use_shell(), use_desktop_ui());
        let env = *ui.env.read();
        let board = use_activity();
        let downloads: Vec<_> = board.queue().into_iter().cloned().collect();
        let summary = match downloads.len() {
            0 => "Nothing downloading".to_string(),
            1 => "1 item".to_string(),
            n => format!("{n} items"),
        };
        let list: Element = if downloads.is_empty() {
            empty_state(&t, "Nothing downloading", "Installs and updates show up here while they run.").into_element()
        } else {
            rect()
                .vertical()
                .spacing(SPACE_2)
                .width(Size::fill())
                .children(downloads.into_iter().map(|d| {
                    // A running job is cancelled by the host; an ended one is just removed from the list.
                    let (id, running, on_effect, dialogs) = (d.id, d.is_running(), shell.on_effect.clone(), ui.dialogs);
                    DownloadItem::new(d)
                        .on_cancel(move |_| on_effect.call(if running { Effect::CancelActivity(id) } else { Effect::DismissActivity(id) }))
                        .on_failure(move |_| dialogs.failure_log(id))
                        .key(id)
                        .into_element()
                }))
                .into_element()
        };
        page_scroll(
            &env,
            rect()
                .vertical()
                .spacing(SPACE_4)
                .width(Size::fill())
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::End)
                        .spacing(SPACE_3)
                        .child(TypeStyle::TitlePage.text("Downloads", t.ink))
                        .child(TypeStyle::Meta.text(summary, t.ink_subtle)),
                )
                .child(list),
        )
    }
}
