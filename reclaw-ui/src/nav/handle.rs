use freya::{prelude::*, router::RouterContext};

use super::{LayerId, Layers, Recents, Route, transition::Direction};

/// How many recent pages are remembered.
const RECENT_PAGES: usize = 12;

/// The one way code outside the router changes page. It wraps `RouterContext` so every navigation
/// records which way it went (the stage animates differently going back) and so tabs, back, forward
/// and "up" behave the same from a click, a key, a mouse button or the pad.
///
/// `Nav` is `Copy`; take it with [`use_nav`] while rendering and move it into handlers. Do not call
/// `RouterContext` or `Link` directly: they skip the direction.
#[derive(Clone, Copy)]
pub struct Nav {
    router: RouterContext,
    direction: State<Direction>,
    recents: State<Recents>,
    layers: State<Layers>,
}

impl Nav {
    /// Create the handle for this router and make it available to descendants. Call once, from the
    /// layout that sits inside the `Router`.
    pub fn use_provide() -> Self {
        let router = RouterContext::get();
        let direction = use_state(|| Direction::Forward);
        let mut recents = use_state(|| Recents::new(RECENT_PAGES));
        let layers = use_state(Layers::default);
        let nav = Self { router, direction, recents, layers };
        use_provide_context(move || nav);
        // Feed the recent-pages list from the router itself so a page change from any source is recorded.
        use_side_effect(move || {
            let route = router.current::<Route>();
            recents.write().visit(&route);
        });
        nav
    }

    /// The page showing now. Reading it in a component re-renders that component on every change.
    pub fn current(&self) -> Route {
        self.router.current::<Route>()
    }

    /// The page showing now, without subscribing: for handlers.
    pub fn here(&self) -> Route {
        self.router.full_route_string().parse().unwrap_or(Route::Library {})
    }

    /// The direction of the latest navigation.
    pub fn direction(&self) -> Direction {
        *self.direction.peek()
    }

    fn set_direction(&self, direction: Direction) {
        let mut state = self.direction;
        state.set(direction);
    }

    /// Open a page. A top-level page replaces the current one (tabs do not grow the back stack);
    /// anything else is pushed on top.
    pub fn open(&self, route: Route) {
        if route == self.here() {
            return;
        }
        if route.is_root() {
            self.set_direction(Direction::Replace);
            self.router.replace(route).ok();
        } else {
            self.set_direction(Direction::Forward);
            self.router.push(route).ok();
        }
    }

    /// Whether `back` would do anything: a dialog to close, history, or a page above this one.
    pub fn can_go_back(&self) -> bool {
        !self.layers.read().is_empty() || self.router.can_go_back() || self.here().up().is_some()
    }

    /// Show a layer over the page (a dialog, a menu). Back closes it before leaving the page.
    /// Pair with [`close_layer`](Self::close_layer) when the owner hides it itself.
    pub fn open_layer(&self) -> LayerId {
        let mut layers = self.layers;
        layers.write().open()
    }

    pub fn close_layer(&self, id: LayerId) {
        let mut layers = self.layers;
        layers.write().close(id);
    }

    /// Whether a layer is still open. Reading it subscribes the component, so it redraws when Back
    /// closes the layer from outside.
    pub fn is_layer_open(&self, id: LayerId) -> bool {
        self.layers.read().contains(id)
    }

    pub fn can_go_forward(&self) -> bool {
        self.router.can_go_forward()
    }

    /// Close the topmost dialog or menu; with none open, go back one page; with no history, up to the
    /// page above (a page opened by a link has no past). Returns false when there was nowhere to go.
    pub fn back(&self) -> bool {
        let mut layers = self.layers;
        if layers.write().close_top() {
            true
        } else if self.router.can_go_back() {
            self.set_direction(Direction::Back);
            self.router.go_back();
            true
        } else if let Some(up) = self.here().up() {
            self.set_direction(Direction::Back);
            self.router.replace(up).ok();
            true
        } else {
            false
        }
    }

    pub fn forward(&self) -> bool {
        if self.router.can_go_forward() {
            self.set_direction(Direction::Forward);
            self.router.go_forward();
            true
        } else {
            false
        }
    }

    /// Recently visited pages, newest first, without the current one.
    pub fn recents(&self) -> Vec<Route> {
        let here = self.current();
        self.recents.read().others(&here)
    }
}

/// The navigation handle. Panics outside an `AppLayout`, which is a bug in the caller.
#[track_caller]
pub fn use_nav() -> Nav {
    use_consume::<Nav>()
}
