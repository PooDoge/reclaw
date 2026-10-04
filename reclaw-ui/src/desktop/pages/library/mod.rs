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
mod wide;

use freya::prelude::*;

use self::ctx::Ctx;
pub use self::filter::Filter;
use crate::{desktop::use_desktop_ui, metrics::*, nav::use_nav, prelude::*, shell::use_shell};

#[derive(PartialEq)]
pub struct LibraryPage {}

impl Component for LibraryPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui, nav) = (use_shell(), use_desktop_ui(), use_nav());
        let env = *ui.env.read();
        let games = shell.host.games.read().clone();
        let downloads = shell.host.downloads.read().clone();
        let selected = ui.selected;

        let ctx = Ctx {
            t,
            env,
            visible: filter::visible(&games, *ui.filter.read(), &ui.search.read()),
            current: games.iter().find(|g| Some(g.id) == *selected.read()).or(games.first()).cloned(),
            installed: games.iter().filter(|g| g.status.is_installed()).count() as u32,
            updates: games.iter().filter(|g| g.status == AppStatus::UpdateReady).count() as u32,
            games,
            downloads,
            selected,
            filter: ui.filter,
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
