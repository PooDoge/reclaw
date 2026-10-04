use super::Route;

/// The pages visited most recently, newest first. This is not the back stack: going back does not
/// remove entries, and visiting a page again moves it to the front. It answers "where was I a
/// minute ago" for a list the user can jump through.
///
/// Router history cannot be read back from `freya-router`, and the back stack is the wrong shape
/// for this anyway, so the list is kept separately and fed every time the current route changes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Recents {
    items: Vec<Route>,
    capacity: usize,
}

impl Recents {
    pub fn new(capacity: usize) -> Self {
        Self { items: Vec::new(), capacity: capacity.max(1) }
    }

    /// Record a visit. Forms are not worth returning to and are skipped.
    pub fn visit(&mut self, route: &Route) {
        if !route.is_recent_worthy() {
            return;
        }
        self.items.retain(|r| r != route);
        self.items.insert(0, route.clone());
        self.items.truncate(self.capacity);
    }

    /// Newest first; the first entry is the page the user is on.
    pub fn items(&self) -> &[Route] {
        &self.items
    }

    /// Everything except `current`, newest first: what a "recent pages" menu lists.
    pub fn others(&self, current: &Route) -> Vec<Route> {
        self.items.iter().filter(|r| *r != current).cloned().collect()
    }

    /// Paths for saving between runs.
    pub fn to_paths(&self) -> Vec<String> {
        self.items.iter().map(ToString::to_string).collect()
    }

    /// Rebuild from saved paths. Entries that no longer parse (a route that was renamed or removed
    /// by an update) are dropped, so an old file can never fail a start.
    pub fn from_paths(capacity: usize, paths: &[String]) -> Self {
        let mut recents = Self::new(capacity);
        for path in paths.iter().rev() {
            if let Ok(route) = path.parse::<Route>() {
                recents.visit(&route);
            }
        }
        recents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: u32) -> Route {
        Route::Game { id }
    }

    #[test]
    fn newest_first_and_revisits_move_to_the_front() {
        let mut r = Recents::new(5);
        for route in [Route::Library {}, game(1), game(2), game(1)] {
            r.visit(&route);
        }
        assert_eq!(r.items(), &[game(1), game(2), Route::Library {}]);
    }

    #[test]
    fn capacity_drops_the_oldest() {
        let mut r = Recents::new(2);
        for id in 1..=4 {
            r.visit(&game(id));
        }
        assert_eq!(r.items(), &[game(4), game(3)]);
    }

    #[test]
    fn forms_are_skipped_and_others_excludes_the_current_page() {
        let mut r = Recents::new(5);
        r.visit(&game(1));
        r.visit(&Route::Install { id: 1 });
        assert_eq!(r.items(), &[game(1)]);
        r.visit(&Route::Mods {});
        assert_eq!(r.others(&Route::Mods {}), vec![game(1)]);
    }

    #[test]
    fn saved_paths_restore_in_order_and_survive_garbage() {
        let mut r = Recents::new(5);
        for route in [Route::Library {}, game(7), Route::Mods {}] {
            r.visit(&route);
        }
        let mut paths = r.to_paths();
        paths.insert(1, "/a/page/that/was/removed".to_string());
        assert_eq!(Recents::from_paths(5, &paths).items(), r.items());
    }
}
