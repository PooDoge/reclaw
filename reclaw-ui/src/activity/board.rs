use super::types::{Activity, ActivityId, Changelog, Kind, Outcome, Stage};

/// How many finished, failed and cancelled rows are kept. Older ones are dropped first.
const KEEP_FINISHED: usize = 50;

/// A change to one activity. Sent by the host as it downloads and builds; nothing else creates or
/// edits an activity. `Send`, so any thread can build one and hand it to the store's feed.
#[derive(Clone, PartialEq, Debug)]
pub enum ActivityEvent {
    Started {
        id: ActivityId,
        game_id: u32,
        kind: Kind,
        title: String,
        bytes_total: Option<u64>,
    },
    Progress {
        id: ActivityId,
        stage: Stage,
        bytes_done: u64,
        bytes_total: Option<u64>,
        rate: Option<u64>,
    },
    Finished {
        id: ActivityId,
        changelog: Option<Changelog>,
    },
    Failed {
        id: ActivityId,
        /// One line, for the row.
        reason: String,
        /// What else to say (the cause, what to do about it), for the notice.
        details: Vec<String>,
    },
    /// The job was cancelled; it leaves the board.
    Cancelled {
        id: ActivityId,
    },
    /// The user removed a finished or failed row from the list.
    Dismiss {
        id: ActivityId,
    },
}

/// What an event did, for the store: which game's views to redraw and what to tell the user.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    Started,
    Progressed,
    Finished,
    Failed,
    Removed,
}

/// How many activities are in each state.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Counts {
    pub running: usize,
    pub failed: usize,
    pub finished: usize,
}

/// Every activity of this run, oldest first.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct ActivityBoard {
    items: Vec<Activity>,
}

impl ActivityBoard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply an event. `None` means it named an activity that is not there (or one that already
    /// ended): late and duplicate events are normal and ignored. Otherwise the game it touched and
    /// what happened.
    pub fn apply(&mut self, event: ActivityEvent) -> Option<(u32, Change)> {
        match event {
            ActivityEvent::Started { id, game_id, kind, title, bytes_total } => {
                if self.get(id).is_some() {
                    return None;
                }
                self.items.push(Activity {
                    id,
                    game_id,
                    kind,
                    title,
                    stage: Stage::Queued,
                    bytes_done: 0,
                    bytes_total,
                    rate: None,
                    changelog: None,
                    details: Vec::new(),
                    outcome: Outcome::Running,
                });
                Some((game_id, Change::Started))
            }
            ActivityEvent::Progress { id, stage, bytes_done, bytes_total, rate } => {
                let a = self.running_mut(id)?;
                a.stage = stage;
                a.bytes_total = bytes_total.or(a.bytes_total);
                a.bytes_done = a.bytes_total.map_or(bytes_done, |total| bytes_done.min(total));
                a.rate = rate;
                Some((a.game_id, Change::Progressed))
            }
            ActivityEvent::Finished { id, changelog } => {
                let a = self.running_mut(id)?;
                a.outcome = Outcome::Finished;
                a.changelog = changelog;
                a.rate = None;
                if let Some(total) = a.bytes_total {
                    a.bytes_done = total;
                }
                let game = a.game_id;
                self.trim();
                Some((game, Change::Finished))
            }
            ActivityEvent::Failed { id, reason, details } => {
                let a = self.running_mut(id)?;
                a.outcome = Outcome::Failed { reason };
                a.details = details;
                a.rate = None;
                let game = a.game_id;
                self.trim();
                Some((game, Change::Failed))
            }
            ActivityEvent::Cancelled { id } => {
                let at = self.items.iter().position(|a| a.id == id && a.is_running())?;
                Some((self.items.remove(at).game_id, Change::Removed))
            }
            ActivityEvent::Dismiss { id } => {
                let at = self.items.iter().position(|a| a.id == id && !a.is_running())?;
                Some((self.items.remove(at).game_id, Change::Removed))
            }
        }
    }

    fn running_mut(&mut self, id: ActivityId) -> Option<&mut Activity> {
        self.items.iter_mut().find(|a| a.id == id && a.is_running())
    }

    /// Past the limit, the oldest ended rows go; running ones never do.
    fn trim(&mut self) {
        while self.items.iter().filter(|a| !a.is_running()).count() > KEEP_FINISHED {
            if let Some(at) = self.items.iter().position(|a| !a.is_running()) {
                self.items.remove(at);
            }
        }
    }

    pub fn get(&self, id: ActivityId) -> Option<&Activity> {
        self.items.iter().find(|a| a.id == id)
    }

    /// Everything on the board, oldest first.
    pub fn all(&self) -> &[Activity] {
        &self.items
    }

    pub fn for_game(&self, game_id: u32) -> impl Iterator<Item = &Activity> {
        self.items.iter().filter(move |a| a.game_id == game_id)
    }

    /// The Downloads list: running first in the order they started, then failed, then finished with
    /// the most recent on top.
    pub fn queue(&self) -> Vec<&Activity> {
        let running = self.items.iter().filter(|a| a.is_running());
        let failed = self.items.iter().filter(|a| matches!(a.outcome, Outcome::Failed { .. }));
        let finished = self.items.iter().rev().filter(|a| a.outcome == Outcome::Finished);
        running.chain(failed).chain(finished).collect()
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for a in &self.items {
            match a.outcome {
                Outcome::Running => counts.running += 1,
                Outcome::Failed { .. } => counts.failed += 1,
                Outcome::Finished => counts.finished += 1,
                Outcome::Cancelled => {}
            }
        }
        counts
    }

    pub fn has_running(&self) -> bool {
        self.items.iter().any(Activity::is_running)
    }
}
