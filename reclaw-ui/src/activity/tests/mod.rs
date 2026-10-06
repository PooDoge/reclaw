//! Tests for the activity board, its indicators and the text formats.
mod board;
mod failure;
mod format;
mod indicator;

use super::*;
use crate::model::ModProvider;

pub(super) fn started(id: ActivityId, game_id: u32, kind: Kind, total: Option<u64>) -> ActivityEvent {
    ActivityEvent::Started { id, game_id, kind, title: format!("job {id}"), bytes_total: total }
}

pub(super) fn progress(id: ActivityId, stage: Stage, done: u64, total: Option<u64>, rate: Option<u64>) -> ActivityEvent {
    ActivityEvent::Progress { id, stage, bytes_done: done, bytes_total: total, rate }
}

pub(super) fn a_mod() -> Kind {
    Kind::Mod { provider: ModProvider::Thunderstore, id: "hd".into() }
}
