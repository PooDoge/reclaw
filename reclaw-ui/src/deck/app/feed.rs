use std::{cell::RefCell, rc::Rc};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;
use reclaw_input::Action;

/// Where gamepad actions arrive. The mounted app borrows the receiving end and hands it back when
/// it unmounts, so Deck mode can be left for the desktop and entered again. The type compares
/// equal to itself so it can sit in props.
#[derive(Clone)]
pub struct ActionFeed {
    rx: Rc<RefCell<Option<UnboundedReceiver<Action>>>>,
    tx: UnboundedSender<Action>,
}

impl ActionFeed {
    pub fn new() -> (UnboundedSender<Action>, Self) {
        let (tx, rx) = unbounded();
        (tx.clone(), Self { rx: Rc::new(RefCell::new(Some(rx))), tx })
    }

    /// A sender for this feed, for code that only holds the feed (the host root, tests).
    pub fn sender(&self) -> UnboundedSender<Action> {
        self.tx.clone()
    }

    /// Borrow the receiving end, dropping presses that piled up while nothing was listening: a
    /// button hit on the desktop must not replay the moment Deck mode opens. `None` if another
    /// mounted app already holds it.
    pub(super) fn take(&self) -> Option<FeedReceiver> {
        let mut rx = self.rx.borrow_mut().take()?;
        while rx.try_recv().is_ok() {}
        Some(FeedReceiver { rx: Some(rx), slot: self.rx.clone() })
    }
}

/// The borrowed receiving end; returns itself to the feed when dropped.
pub(super) struct FeedReceiver {
    rx: Option<UnboundedReceiver<Action>>,
    slot: Rc<RefCell<Option<UnboundedReceiver<Action>>>>,
}

impl FeedReceiver {
    pub async fn next(&mut self) -> Option<Action> {
        self.rx.as_mut()?.next().await
    }
}

impl Drop for FeedReceiver {
    fn drop(&mut self) {
        *self.slot.borrow_mut() = self.rx.take();
    }
}

impl PartialEq for ActionFeed {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt;

    use super::*;

    #[test]
    fn the_receiver_comes_back_when_dropped_and_stale_presses_are_discarded() {
        let (tx, feed) = ActionFeed::new();
        let mut first = feed.take().expect("free at first");
        assert!(feed.take().is_none(), "only one listener at a time");
        tx.unbounded_send(Action::Confirm).expect("the feed is open");
        assert_eq!(first.next().now_or_never(), Some(Some(Action::Confirm)));
        drop(first);

        tx.unbounded_send(Action::Back).expect("the feed is open"); // pressed while nothing listened
        let mut second = feed.take().expect("returned on drop");
        assert_eq!(second.next().now_or_never(), None, "the stale press was dropped, nothing is pending");
        tx.unbounded_send(Action::Confirm).expect("the feed is open");
        assert_eq!(second.next().now_or_never(), Some(Some(Action::Confirm)));
    }
}
