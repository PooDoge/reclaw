use freya::prelude::*;
use reclaw_input::FocusId;

use super::empty::EmptyPage;
use crate::{deck::ids, metrics::*, prelude::*, typography::TypeStyle};

/// Download queue: one `DownloadItem` per entry with its Cancel as the focus target.
#[derive(Clone, PartialEq)]
pub struct DownloadsPage {
    downloads: Vec<Download>,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl DownloadsPage {
    pub fn new(downloads: Vec<Download>, focus: FocusId, ring_visible: bool, on_click: EventHandler<FocusId>) -> Self {
        Self { downloads, focus, ring_visible, on_click }
    }
}

impl Component for DownloadsPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        if self.downloads.is_empty() {
            return EmptyPage::new("Nothing in the queue", "Installs and updates show up here.").into_element();
        }
        let items = self.downloads.iter().map(|d| {
            let id = ids::download_cancel(d.app_id);
            let on_click = self.on_click.clone();
            DownloadItem::new(d.clone())
                .controller(true, self.ring_visible && self.focus == id)
                .on_cancel(move |_| on_click.call(id))
                .key(d.app_id)
                .into_element()
        });
        rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::fill())
            .child(TypeStyle::DeckHeading.text("Downloads", t.ink))
            .children(items)
            .into_element()
    }
}
