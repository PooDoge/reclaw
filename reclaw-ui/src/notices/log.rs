//! Every notice the user is shown is also in the log, with its details, once, from the one place all actions pass. A failure the
//! user saw and a failure the log has are then the same set, and a report of "it showed an error" can be matched to a line.
use super::{Notice, NoticeKind};

/// Write `notice` to the log at a level that matches how much the user needs to do about it.
pub fn log(notice: &Notice) {
    let (title, body, details) = (notice.title.as_str(), notice.body.as_str(), notice.details.join(" | "));
    match notice.kind {
        NoticeKind::Problem | NoticeKind::DownloadFailed => {
            tracing::warn!(game = notice.game_id, title, body, details, "problem shown to the user")
        }
        NoticeKind::Note => tracing::info!(title, body, details, "note shown to the user"),
        NoticeKind::UpdateAvailable | NoticeKind::UpdateFinished | NoticeKind::InstallFinished | NoticeKind::ModInstalled => {
            tracing::debug!(game = notice.game_id, title, body, "notification shown to the user");
        }
    }
}
