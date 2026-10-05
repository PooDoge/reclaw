use crate::activity::Changelog;

pub type NoticeId = u64;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NoticeKind {
    UpdateAvailable,
    UpdateFinished,
    InstallFinished,
    DownloadFailed,
    ModInstalled,
    /// Something the user should know that is not a failure: saved data is being shown, a feature is not built yet. Passes.
    Note,
    /// Something went wrong that the user can act on. Stays until dismissed.
    Problem,
}

impl NoticeKind {
    /// A failure is not dismissed by the timeout: it stays until the user deals with it.
    pub fn is_failure(self) -> bool {
        matches!(self, Self::DownloadFailed | Self::Problem)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Notice {
    pub id: NoticeId,
    pub kind: NoticeKind,
    pub game_id: Option<u32>,
    /// "Skyward Quest updated".
    pub title: String,
    /// One line under the title: "v0.9.1 to v0.9.2", "Release asset not found".
    pub body: String,
    /// What the details view lists: the changelog lines, the failure's reason.
    pub details: Vec<String>,
    /// Where the full release notes are, when there is one.
    pub url: Option<String>,
}

impl Notice {
    pub(super) fn new(kind: NoticeKind, game_id: Option<u32>, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { id: 0, kind, game_id, title: title.into(), body: body.into(), details: Vec::new(), url: None }
    }

    pub fn details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }

    pub fn url(mut self, url: Option<String>) -> Self {
        self.url = url;
        self
    }

    /// A notice for a finished update, with the release notes as its details.
    pub fn update_finished(game_id: u32, title: &str, changelog: Option<&Changelog>) -> Self {
        let body = changelog.map_or_else(|| "Update installed".to_string(), |c| format!("{} to {}", c.from, c.to));
        let mut notice = Self::new(NoticeKind::UpdateFinished, Some(game_id), format!("{title} updated"), body);
        if let Some(c) = changelog {
            notice = notice.details(c.lines().into_iter().map(str::to_string).collect()).url(c.url.clone());
        }
        notice
    }

    pub fn update_available(game_id: u32, title: &str) -> Self {
        Self::new(NoticeKind::UpdateAvailable, Some(game_id), format!("Update ready for {title}"), "Open it to update")
    }

    pub fn install_finished(game_id: u32, title: &str) -> Self {
        Self::new(NoticeKind::InstallFinished, Some(game_id), format!("{title} is ready"), "Installed")
    }

    pub fn download_failed(game_id: u32, title: &str, reason: &str) -> Self {
        Self::new(NoticeKind::DownloadFailed, Some(game_id), format!("{title} failed"), reason).details(vec![reason.to_string()])
    }

    pub fn note(title: &str, body: &str, details: Vec<String>) -> Self {
        Self::new(NoticeKind::Note, None, title, body).details(details)
    }

    pub fn problem(title: &str, body: &str, details: Vec<String>) -> Self {
        Self::new(NoticeKind::Problem, None, title, body).details(details)
    }

    pub fn mod_installed(game_id: u32, title: &str) -> Self {
        Self::new(NoticeKind::ModInstalled, Some(game_id), format!("{title} installed"), "Mod ready")
    }
}
