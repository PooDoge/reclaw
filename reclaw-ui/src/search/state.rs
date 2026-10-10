//! The search box's state: whether it is open, and for each tab what is typed and what was searched.
//!
//! The box opens on the button and closes again when a search is sent or it loses focus. What was typed but not sent is kept
//! when it closes, so a stray click does not lose it; it is dropped once it has sat unsent for [`DRAFT_KEPT_SECS`], and the box
//! then opens on the search that is running (or empty). The results a tab shows follow only what was sent.
use super::scope::SearchScope;

/// How long text typed but never sent is kept after the box closes.
pub const DRAFT_KEPT_SECS: u64 = 120;

/// One tab's search.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ScopeSearch {
    /// What the box held when it last closed.
    pub draft: String,
    /// The search that is running: what the tab's results follow. Empty is none.
    pub query: String,
    /// When the box closed with text that was never sent (seconds since 1970).
    pub held_since: Option<u64>,
}

impl ScopeSearch {
    /// Whether the box holds text that differs from the running search.
    pub fn holds_draft(&self) -> bool {
        self.draft.trim() != self.query
    }
}

/// Every tab's search, and which one the box is open for.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SearchModel {
    scopes: [ScopeSearch; 4],
    open: Option<SearchScope>,
}

impl SearchModel {
    pub fn scope(&self, scope: SearchScope) -> &ScopeSearch {
        &self.scopes[scope.index()]
    }

    fn scope_mut(&mut self, scope: SearchScope) -> &mut ScopeSearch {
        &mut self.scopes[scope.index()]
    }

    /// The search a tab's results follow; empty when there is none.
    pub fn query(&self, scope: SearchScope) -> &str {
        &self.scope(scope).query
    }

    /// The tab the box is open for, if it is open.
    pub fn open_for(&self) -> Option<SearchScope> {
        self.open
    }

    /// Open the box for `scope`, and say what it should hold: the text kept from last time, unless it has sat unsent too long,
    /// then the running search.
    pub fn open(&mut self, scope: SearchScope, now: u64) -> String {
        let s = self.scope_mut(scope);
        if s.held_since.is_some_and(|since| now.saturating_sub(since) >= DRAFT_KEPT_SECS) {
            s.draft = s.query.clone();
            s.held_since = None;
        }
        self.open = Some(scope);
        self.scope(scope).draft.clone()
    }

    /// The box lost focus: close it and keep what it holds. Nothing changes when it was not open.
    pub fn close(&mut self, text: &str, now: u64) {
        let Some(scope) = self.open.take() else { return };
        let s = self.scope_mut(scope);
        s.draft = text.to_string();
        s.held_since = s.holds_draft().then_some(now);
    }

    /// The search was sent: it becomes the tab's running search, and the box closes. Returns the tab and the search, or `None`
    /// when the box was not open. An empty search ends the running one.
    pub fn submit(&mut self, text: &str) -> Option<(SearchScope, String)> {
        let scope = self.open.take()?;
        let s = self.scope_mut(scope);
        s.query = text.trim().to_string();
        s.draft = s.query.clone();
        s.held_since = None;
        Some((scope, s.query.clone()))
    }

    /// End a tab's search: its results show everything again and the box opens empty.
    pub fn clear(&mut self, scope: SearchScope) {
        *self.scope_mut(scope) = ScopeSearch::default();
    }

    /// Set a tab's running search from outside the box (a link that opens a tab with a search, or none).
    pub fn set_query(&mut self, scope: SearchScope, query: &str) {
        let s = self.scope_mut(scope);
        s.query = query.trim().to_string();
        s.draft = s.query.clone();
        s.held_since = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: u64 = 1_000;

    #[test]
    fn a_sent_search_runs_and_the_box_closes() {
        let mut m = SearchModel::default();
        assert_eq!(m.open(SearchScope::Mods, T), "");
        assert_eq!(m.open_for(), Some(SearchScope::Mods));
        assert_eq!(m.submit("  camera "), Some((SearchScope::Mods, "camera".to_string())));
        assert_eq!((m.open_for(), m.query(SearchScope::Mods)), (None, "camera"));
        assert_eq!(m.query(SearchScope::Library), "", "each tab has its own search");
        assert_eq!(m.open(SearchScope::Mods, T + 9999), "camera", "reopening shows the running search");
    }

    #[test]
    fn text_typed_and_left_is_kept_for_a_while_then_dropped() {
        let mut m = SearchModel::default();
        m.open(SearchScope::Library, T);
        m.close("zel", T);
        assert_eq!(m.query(SearchScope::Library), "", "leaving the box does not search");
        assert!(m.scope(SearchScope::Library).holds_draft());
        assert_eq!(m.open(SearchScope::Library, T + DRAFT_KEPT_SECS - 1), "zel", "a stray click loses nothing");
        m.close("zelda", T + 200);
        assert_eq!(m.open(SearchScope::Library, T + 200 + DRAFT_KEPT_SECS), "", "left too long, it is dropped");
    }

    #[test]
    fn a_dropped_draft_falls_back_to_the_running_search() {
        let mut m = SearchModel::default();
        m.open(SearchScope::Catalog, T);
        m.submit("mario");
        m.open(SearchScope::Catalog, T);
        m.close("mario kart", T);
        assert_eq!(m.query(SearchScope::Catalog), "mario");
        assert_eq!(m.open(SearchScope::Catalog, T + DRAFT_KEPT_SECS), "mario");
    }

    #[test]
    fn closing_on_the_running_search_holds_nothing() {
        let mut m = SearchModel::default();
        m.open(SearchScope::Settings, T);
        m.submit("theme");
        m.open(SearchScope::Settings, T);
        m.close("theme", T);
        assert_eq!(m.scope(SearchScope::Settings).held_since, None);
    }

    #[test]
    fn sending_nothing_ends_the_search_and_clear_forgets_everything() {
        let mut m = SearchModel::default();
        m.open(SearchScope::Mods, T);
        m.submit("x");
        m.open(SearchScope::Mods, T);
        assert_eq!(m.submit("   "), Some((SearchScope::Mods, String::new())));
        m.set_query(SearchScope::Mods, "y");
        assert_eq!(m.query(SearchScope::Mods), "y");
        m.clear(SearchScope::Mods);
        assert_eq!(m.scope(SearchScope::Mods), &ScopeSearch::default());
        assert_eq!(m.submit("z"), None, "nothing is sent while the box is closed");
        m.close("ignored", T);
        assert_eq!(m.scope(SearchScope::Mods).draft, "");
    }
}
