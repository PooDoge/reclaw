//! The Game page: one project's banner and action bar, then what to know before playing it: about,
//! screenshots and videos, recent updates from the project, requirements, the mods made for it.
//!
//! * `ctx`: everything the sections share, built once per render
//! * `hero`: the banner and the actions (install or play, favorite, settings); `about`, `media`,
//!   `updates`, `facts`, `mods`: one section each
//!
//! Pure data comes from `crate::catalog::GameView`; nothing here decides, it only lays out.
mod about;
mod ctx;
mod facts;
mod hero;
mod media;
mod mods;
mod updates;

use freya::prelude::*;

use self::ctx::Ctx;
use super::common::{BackBar, NotFound, page_scroll};
use crate::{catalog::GameView, desktop::use_desktop_ui, metrics::*, nav::use_nav, nav::use_page_motion, prelude::*, shell::use_shell};

#[derive(PartialEq)]
pub struct GamePage {
    pub id: u32,
}

impl Component for GamePage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui, nav) = (use_shell(), use_desktop_ui(), use_nav());
        let motion = use_page_motion();
        let env = *ui.env.read();
        let view = GameView::resolve(self.id, &shell.host.games.read(), &shell.host.projects.read(), &shell.host.mods.read());
        let Some(view) = view else {
            return NotFound { what: "game", id: self.id.to_string() }.into_element();
        };
        let c = Ctx { t, env, view, nav, dialogs: ui.dialogs, on_effect: shell.on_effect.clone() };

        // After the banner has settled, the sections below it rise in one after another.
        let section = |index: usize, content: Option<Element>| {
            let (opacity, dy) = motion.rise(0.45 + 0.1 * index as f32, 1., 20.);
            content.map(|el| rect().width(Size::fill()).opacity(opacity).offset_y(dy).child(el).into_element())
        };

        let wide = env.class == LayoutClass::Wide;
        let main = [section(0, about::view(&c)), section(1, media::view(&c)), section(2, updates::view(&c)), section(3, mods::view(&c))];
        let side = [section(1, facts::details(&c)), section(2, facts::requirements(&c)), section(3, facts::links(&c))];

        let body = if wide {
            rect()
                .horizontal()
                .content(Content::Flex)
                .spacing(SPACE_5)
                .width(Size::fill())
                .child(rect().vertical().spacing(SPACE_5).width(Size::flex(1.)).children(main.into_iter().flatten()))
                .child(rect().vertical().spacing(SPACE_5).width(Size::px(300.)).children(side.into_iter().flatten()))
        } else {
            let [about, media, updates, mods] = main;
            let [details, requirements, links] = side;
            rect()
                .vertical()
                .spacing(SPACE_5)
                .width(Size::fill())
                .children([about, media, updates, details, requirements, mods, links].into_iter().flatten())
        };

        let narrow = env.class != LayoutClass::Wide;
        page_scroll(
            &env,
            rect().vertical().spacing(SPACE_5).width(Size::fill()).child(BackBar {}).child(hero::view(&c, narrow)).child(body),
        )
    }
}
