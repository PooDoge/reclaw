use super::types::{Notice, NoticeId, NoticeKind};

/// How many notices are kept; older ones are dropped.
const KEEP: usize = 30;

/// The notices waiting for the user, oldest first. The newest is the one on screen.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Notices {
    items: Vec<Notice>,
    next: NoticeId,
}

impl Notices {
    /// Add a notice. A notice of the same kind about the same game replaces the old one, so a
    /// download that fails three times is one notice, not three. Returns its id.
    pub fn push(&mut self, mut notice: Notice) -> NoticeId {
        self.items.retain(|n| !(n.kind == notice.kind && n.game_id == notice.game_id));
        // An update that finished makes "update ready" for the same game stale.
        if notice.kind == NoticeKind::UpdateFinished || notice.kind == NoticeKind::InstallFinished {
            self.items.retain(|n| !(n.kind == NoticeKind::UpdateAvailable && n.game_id == notice.game_id));
        }
        self.next += 1;
        notice.id = self.next;
        let id = notice.id;
        self.items.push(notice);
        if self.items.len() > KEEP {
            self.items.remove(0);
        }
        id
    }

    /// The one to show: the newest.
    pub fn top(&self) -> Option<&Notice> {
        self.items.last()
    }

    pub fn get(&self, id: NoticeId) -> Option<&Notice> {
        self.items.iter().find(|n| n.id == id)
    }

    /// Newest first, for a list.
    pub fn all(&self) -> impl Iterator<Item = &Notice> {
        self.items.iter().rev()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn dismiss(&mut self, id: NoticeId) -> bool {
        let before = self.items.len();
        self.items.retain(|n| n.id != id);
        self.items.len() != before
    }

    /// Dismiss everything. Returns how many there were.
    pub fn dismiss_all(&mut self) -> usize {
        std::mem::take(&mut self.items).len()
    }

    /// Dismiss the notices about one game (it was opened, or removed).
    pub fn dismiss_game(&mut self, game_id: u32) {
        self.items.retain(|n| n.game_id != Some(game_id));
    }
}
