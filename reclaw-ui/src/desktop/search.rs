//! The desktop's search box: in the top bar's right corner on a wide window, floating at the top right of the page on a narrow
//! one. It searches the tab that is showing (`search::SearchScope`), and its state is the frame's (`DesktopUi::search`).
//!
//! Focus decides open and closed: the field gaining focus (a press on the button, Tab) opens the box with what it held last
//! time, and losing it (a click elsewhere, Escape, another tab) closes it and keeps what was typed. Enter or the magnifier sends
//! the search, closes the box and shows the results on the tab's page.
use freya::prelude::*;

use super::use_desktop_ui;
use crate::{components::SearchToggle, nav::use_nav, search::SearchScope, shell::use_shell, store::now_secs};

#[derive(PartialEq)]
pub struct DesktopSearch {
    /// The tab being searched; `None` where there is nothing to search (Downloads): the button is then not drawn, but its
    /// space is kept so the bar beside it does not move.
    pub scope: Option<SearchScope>,
    /// What the box's placeholder narrows to, such as the game the Mods tab shows.
    pub within: Option<String>,
    /// The open width.
    pub expanded: f32,
    /// The button's side.
    pub size: f32,
}

impl Component for DesktopSearch {
    fn render(&self) -> impl IntoElement {
        let (ui, nav, shell) = (use_desktop_ui(), use_nav(), use_shell());
        let animated = !shell.transitions.read().reduce_motion;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let scope = use_reactive(&self.scope);
        let (mut model, mut field, mut results_page, mut shelf) = (ui.search, ui.search_field, ui.mod_results_page, ui.mod_shelf);

        use_side_effect(move || {
            let focused = focus().is_focused();
            let open = model.peek().open_for();
            match (focused, open, *scope.peek()) {
                (true, None, Some(scope)) => {
                    let text = model.write().open(scope, now_secs());
                    field.set(text);
                }
                // Nothing to search here: a Tab that lands on the hidden field moves on.
                (true, None, None) => a11y.request_unfocus(),
                (false, Some(_), _) => {
                    let text = field.peek().clone();
                    model.write().close(&text, now_secs());
                }
                _ => {}
            }
        });

        let Some(current) = self.scope else {
            return rect().width(Size::px(self.size)).height(Size::px(self.size)).into_element();
        };
        let on_submit = move |text: String| {
            let Some((scope, query)) = model.write().submit(&text) else { return };
            field.set(query);
            a11y.request_unfocus();
            if scope == SearchScope::Mods {
                results_page.set(0);
                shelf.set(None);
            }
            let on_page = match scope {
                SearchScope::Settings => SearchScope::root_of(&nav.current()) == Some(SearchScope::Settings),
                _ => nav.current() == scope.route(),
            };
            if !on_page {
                nav.open(scope.route());
            }
        };
        let (open, active) = {
            let m = model.read();
            (m.open_for() == Some(current), !m.query(current).is_empty())
        };
        SearchToggle::new(field, a11y)
            .open(open)
            .active(active)
            .placeholder(current.placeholder(self.within.as_deref()))
            .expanded(self.expanded)
            .size(self.size)
            .animated(animated)
            .on_submit(on_submit)
            .into_element()
    }
}

/// The button's side at a density: the top bar's height allows 32; a narrow window's floating button is a full touch target.
pub fn button_size(class: crate::metrics::LayoutClass, density: crate::metrics::Density) -> f32 {
    use crate::metrics::{Density, LayoutClass, TARGET_MIN};
    match (class, density) {
        (LayoutClass::Wide, _) => 32.,
        (_, Density::Pointer) => 36.,
        _ => TARGET_MIN,
    }
}

/// The open width in the wide window's top bar. It covers the Recent and Deck mode buttons while open rather than moving them.
pub const OPEN_WIDTH: f32 = 320.;
