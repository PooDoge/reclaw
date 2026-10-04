use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;

use super::action::AppAction;

/// How code on other threads (the supervisor, a downloader, a file watcher) changes the store. The
/// store itself lives on the UI thread and cannot be sent; actions can.
#[derive(Clone)]
pub struct StoreFeed {
    tx: UnboundedSender<AppAction>,
}

impl StoreFeed {
    /// Queue an action. `false` if the app has already shut down (nobody is listening any more).
    pub fn send(&self, action: AppAction) -> bool {
        self.tx.unbounded_send(action).is_ok()
    }
}

/// The receiving end, drained by [`Store::pump`](super::Store::pump).
pub struct StoreInbox {
    rx: UnboundedReceiver<AppAction>,
}

impl StoreInbox {
    pub(super) async fn next(&mut self) -> Option<AppAction> {
        self.rx.next().await
    }
}

pub fn feed() -> (StoreFeed, StoreInbox) {
    let (tx, rx) = unbounded();
    (StoreFeed { tx }, StoreInbox { rx })
}
