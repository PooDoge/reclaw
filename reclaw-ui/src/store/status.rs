//! What the catalog loader is doing and has done, for the line under the Catalog's title.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CatalogPhase {
    /// Nothing started yet (or the host has no network layer).
    #[default]
    Idle,
    /// A refresh is running. What is shown meanwhile is the saved copy, if there is one.
    Loading,
    Ready,
    /// The catalog could not be loaded and there is no saved copy to show.
    Failed,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CatalogStatus {
    pub phase: CatalogPhase,
    /// Unix seconds when the oldest part of what is shown was last confirmed by its server.
    pub updated_at: Option<u64>,
    /// What is shown includes a saved copy standing in for a part that could not be fetched.
    pub stale: bool,
    /// What went wrong, in words, one line each.
    pub problems: Vec<String>,
}

/// "just now", "5 min ago", "3 h ago", "2 days ago".
fn ago(now: u64, at: u64) -> String {
    let minutes = now.saturating_sub(at) / 60;
    match minutes {
        0 => "just now".to_string(),
        1..=59 => format!("{minutes} min ago"),
        60..=2879 => format!("{} h ago", minutes / 60),
        _ => format!("{} days ago", minutes / 1440),
    }
}

/// Unix seconds now; the page passes this to [`CatalogStatus::line`] so the line itself stays a pure function.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl CatalogStatus {
    pub fn loading(previous: &Self) -> Self {
        Self { phase: CatalogPhase::Loading, ..previous.clone() }
    }

    /// The short line in the window's bottom strip: what the catalog is doing or how fresh it is.
    pub fn strip(&self, now: u64) -> String {
        match (self.phase, self.updated_at) {
            (CatalogPhase::Idle, None) => "Catalog not loaded yet".to_string(),
            (CatalogPhase::Loading, _) => "Refreshing the catalog...".to_string(),
            (CatalogPhase::Failed, _) => "Catalog unavailable".to_string(),
            (_, Some(at)) if self.stale => format!("Offline: catalog from {}", ago(now, at)),
            (_, Some(at)) => format!("Catalog updated {}", ago(now, at)),
            (_, None) => "Catalog ready".to_string(),
        }
    }

    /// The line for the page: how many projects, how fresh, and whether a refresh is running.
    pub fn line(&self, projects: usize, now: u64) -> String {
        let count = format!("{projects} recompilation projects");
        match (self.phase, self.updated_at) {
            (CatalogPhase::Loading, _) => format!("{count} \u{b7} refreshing..."),
            (CatalogPhase::Failed, _) => "The catalog could not be loaded".to_string(),
            (_, Some(at)) if self.stale => format!("{count} \u{b7} offline, saved copy from {}", ago(now, at)),
            (_, Some(at)) => format!("{count} \u{b7} updated {}", ago(now, at)),
            (_, None) => count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_says_how_fresh_and_whether_it_is_a_saved_copy() {
        let ready = |updated_at, stale| CatalogStatus { phase: CatalogPhase::Ready, updated_at, stale, problems: vec![] };
        assert_eq!(ready(Some(1000), false).line(232, 1030), "232 recompilation projects \u{b7} updated just now");
        assert_eq!(ready(Some(1000), false).line(232, 1000 + 5 * 60), "232 recompilation projects \u{b7} updated 5 min ago");
        assert_eq!(
            ready(Some(1000), true).line(232, 1000 + 3 * 3600),
            "232 recompilation projects \u{b7} offline, saved copy from 3 h ago"
        );
        assert_eq!(ready(Some(1000), false).line(232, 1000 + 3 * 86400), "232 recompilation projects \u{b7} updated 3 days ago");
        assert_eq!(ready(None, false).line(7, 0), "7 recompilation projects");
        assert_eq!(CatalogStatus::loading(&ready(Some(1), false)).line(232, 100), "232 recompilation projects \u{b7} refreshing...");
        assert_eq!(CatalogStatus { phase: CatalogPhase::Failed, ..Default::default() }.line(0, 0), "The catalog could not be loaded");
    }

    #[test]
    fn the_strip_says_what_the_catalog_is_doing() {
        let at = |phase, updated_at, stale| CatalogStatus { phase, updated_at, stale, problems: vec![] };
        assert_eq!(CatalogStatus::default().strip(0), "Catalog not loaded yet");
        assert_eq!(at(CatalogPhase::Loading, Some(1), false).strip(100), "Refreshing the catalog...");
        assert_eq!(at(CatalogPhase::Failed, None, false).strip(100), "Catalog unavailable");
        assert_eq!(at(CatalogPhase::Ready, Some(0), false).strip(7200), "Catalog updated 2 h ago");
        assert_eq!(at(CatalogPhase::Ready, Some(0), true).strip(600), "Offline: catalog from 10 min ago");
    }
}
