use super::{Route, transition::Spec};

/// What the stage is showing: one page at rest, or two while one gives way to the other.
/// Pure so the hand-over rules are tested without a window.
#[derive(Clone, PartialEq, Debug)]
pub struct StageState {
    pub to: Route,
    /// The page being left, while a transition plays.
    pub from: Option<Route>,
    pub spec: Spec,
    /// Changes with every transition so the animation restarts even when the spec is the same.
    pub generation: u64,
}

impl StageState {
    pub fn settled(route: Route, spec: Spec) -> Self {
        Self { to: route, from: None, spec, generation: 0 }
    }

    /// Start showing `route`. A cut (`spec.is_none()`) leaves nothing to animate. Navigating again
    /// mid-transition drops the page that was still fading out: the user has already moved on.
    /// Returns false when `route` is already the target.
    pub fn go(&mut self, route: Route, spec: Spec) -> bool {
        if route == self.to {
            return false;
        }
        let left = std::mem::replace(&mut self.to, route);
        self.from = (!spec.is_none()).then_some(left);
        self.spec = spec;
        self.generation += 1;
        true
    }

    /// The transition finished: only the target remains.
    pub fn settle(&mut self) {
        self.from = None;
    }

    pub fn is_animating(&self) -> bool {
        self.from.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::nav::transition::{Direction, Easing, TransitionStyle};

    fn slide() -> Spec {
        Spec {
            style: TransitionStyle::Slide,
            direction: Direction::Forward,
            duration: Duration::from_millis(200),
            distance: 24.,
            easing: Easing::ExpoOut,
        }
    }

    fn start() -> StageState {
        StageState::settled(Route::Library {}, Spec::none(Direction::Forward))
    }

    #[test]
    fn a_transition_keeps_the_old_page_until_it_settles() {
        let mut stage = start();
        assert!(stage.go(Route::Catalog {}, slide()));
        assert_eq!((&stage.to, &stage.from), (&Route::Catalog {}, &Some(Route::Library {})));
        assert!(stage.is_animating());
        stage.settle();
        assert_eq!(stage.from, None);
    }

    #[test]
    fn a_cut_keeps_nothing_and_still_counts_as_a_change() {
        let mut stage = start();
        let before = stage.generation;
        assert!(stage.go(Route::Catalog {}, Spec::none(Direction::Forward)));
        assert!(!stage.is_animating());
        assert_eq!(stage.generation, before + 1);
    }

    #[test]
    fn going_to_the_current_page_does_nothing() {
        let mut stage = start();
        assert!(!stage.go(Route::Library {}, slide()));
        assert_eq!(stage.generation, 0);
    }

    #[test]
    fn navigating_mid_transition_leaves_the_page_just_reached() {
        let mut stage = start();
        stage.go(Route::Catalog {}, slide());
        stage.go(Route::Mods {}, slide());
        assert_eq!((&stage.to, &stage.from), (&Route::Mods {}, &Some(Route::Catalog {})));
    }
}
