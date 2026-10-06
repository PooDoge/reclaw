//! What a "Failed" indicator says when hovered and what it opens when pressed: a short reason, and a report with the job's log.
use super::{
    board::ActivityBoard,
    types::{Activity, ActivityId, Outcome},
};

/// The most characters of a hover reason. A tooltip is one line; the full text is a press away.
pub const HINT_CHARS: usize = 80;

/// Shown when a game is marked failed but this run has no job for it: it failed before the last restart, and the board does not
/// survive one.
pub const EARLIER_FAILURE: &str = "The last install did not finish. Its log is in the log folder.";

/// What to put behind one "Failed" indicator.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FailureHint {
    /// The job whose report a press opens; `None` when it is from an earlier run (a press then opens the log folder).
    pub activity: Option<ActivityId>,
    /// One line for the tooltip.
    pub text: String,
}

/// Everything known about one failed job, for the log view.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FailureReport {
    pub title: String,
    pub reason: String,
    /// The cause, what to do about it, where the log is.
    pub details: Vec<String>,
    pub log: Vec<String>,
}

/// `reason` cut to [`HINT_CHARS`] at a word boundary, with "..." when anything was cut. Only the first line counts.
pub fn short_reason(reason: &str) -> String {
    let line = reason.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    if line.chars().count() <= HINT_CHARS {
        return line.to_string();
    }
    let room = HINT_CHARS - 3;
    let cut: String = line.chars().take(room).collect();
    // Back to the last space, unless that throws away most of the text (one long path or URL).
    let at = cut.rfind(' ').filter(|at| *at >= room / 2).unwrap_or(cut.len());
    format!("{}...", cut[..at].trim_end_matches([' ', ',', ';', ':', '.']))
}

/// The newest failed job of a game in this run.
pub fn latest_failure(board: &ActivityBoard, game_id: u32) -> Option<&Activity> {
    board.for_game(game_id).filter(|a| matches!(a.outcome, Outcome::Failed { .. })).last()
}

/// The hint for a job that failed.
pub fn hint_for_activity(activity: &Activity) -> FailureHint {
    let text = match &activity.outcome {
        Outcome::Failed { reason } => short_reason(reason),
        _ => String::new(),
    };
    FailureHint { activity: Some(activity.id), text }
}

/// The hint for a game shown as failed: its newest failed job, or the earlier-run text.
pub fn hint_for_game(board: &ActivityBoard, game_id: u32) -> FailureHint {
    latest_failure(board, game_id)
        .map(hint_for_activity)
        .unwrap_or_else(|| FailureHint { activity: None, text: EARLIER_FAILURE.to_string() })
}

/// The report for a failed job; `None` for a job that did not fail or is no longer on the board.
pub fn report(board: &ActivityBoard, id: ActivityId) -> Option<FailureReport> {
    let a = board.get(id)?;
    let Outcome::Failed { reason } = &a.outcome else { return None };
    Some(FailureReport { title: a.title.clone(), reason: reason.clone(), details: a.details.clone(), log: a.log.clone() })
}

/// How a recorded line is drawn: problems stand out from the steps around them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineTone {
    Plain,
    Warning,
    Error,
}

/// From the level written after the time (`05:42:00.123  WARN reclaw_net: ...`).
pub fn line_tone(line: &str) -> LineTone {
    match line.split_whitespace().nth(1) {
        Some("ERROR") => LineTone::Error,
        Some("WARN") => LineTone::Warning,
        _ => LineTone::Plain,
    }
}
