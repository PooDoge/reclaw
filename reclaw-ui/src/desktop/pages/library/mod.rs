//! The Library page at all three layout classes. Wide: sidebar list + hero + downloads. Compact:
//! hero + capsule grid. Phone: capsule grid. Opening a game is a route (`/game/:id`); the dialogs
//! and the navigation bars belong to the desktop frame.
//!
//! * `filter`: which games show (pure, tested); `ctx`: what the layouts share
//! * `wide` / `compact` / `phone`: one file per layout; `parts`: pieces more than one uses
mod compact;
mod ctx;
mod filter;
mod parts;
mod phone;
mod updates;
mod wide;

use freya::prelude::*;

use self::ctx::Ctx;
pub use self::filter::Filter;
use crate::{
    activity::sidebar_entries,
    desktop::use_desktop_ui,
    metrics::*,
    nav::use_nav,
    prelude::*,
    shell::use_shell,
    store::{use_activity, use_games, use_settings},
    systems::{Sort, systems_in},
};

#[derive(PartialEq)]
pub struct LibraryPage {}

impl Component for LibraryPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui, nav) = (use_shell(), use_desktop_ui(), use_nav());
        let env = *ui.env.read();
        let (games, activity, settings) = (use_games(), use_activity(), use_settings());
        let sort = Sort::from_settings(&settings);
        let selected = ui.selected;
        let updates_section = sidebar_entries(&activity, &games);

        let systems = systems_in(&games);
        let visible = filter::visible(&games, *ui.filter.read(), *ui.system.read(), &ui.search.read(), sort);
        let ctx = Ctx {
            t,
            env,
            // The hero shows the selected game if the filters leave it in the list, else the first one left.
            current: visible.iter().find(|g| Some(g.id) == *selected.read()).or(visible.first()).cloned(),
            visible,
            installed: games.iter().filter(|g| g.status.is_installed()).count() as u32,
            updates: games.iter().filter(|g| g.status == AppStatus::UpdateReady).count() as u32,
            games,
            updates_section,
            activity,
            selected,
            filter: ui.filter,
            system: ui.system,
            systems,
            sort,
            search: ui.search,
            dialogs: ui.dialogs,
            nav,
            on_effect: shell.on_effect.clone(),
            toggle_deck: {
                let mut model = shell.model;
                EventHandler::new(move |()| model.write().toggle_mode())
            },
        };
        match env.class {
            LayoutClass::Wide => wide::layout(&ctx),
            LayoutClass::Compact => compact::layout(&ctx),
            LayoutClass::Phone => phone::layout(&ctx),
        }
    }
}
