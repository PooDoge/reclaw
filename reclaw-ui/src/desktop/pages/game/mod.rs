//! The Game page: one project's banner and action bar, then what to know before playing it: about,
//! screenshots and videos, recent updates from the project, requirements, the mods made for it and a link to all of them.
//!
//! * `ctx`: everything the sections share, built once per render
//! * `hero`: the banner and the actions (install or play, favorite, settings); `about`, `media`,
//!   `updates`, `facts`, `mods`, `readme`: one section each
//! * `community`: what quiverlauncher.com says (how it runs and player reviews, its releases as the site judged them, who made
//!   it); the site's releases take the place of `updates` once they are read
//!
//! Pure data comes from `crate::catalog::GameView`; nothing here decides, it only lays out.
mod about;
mod community;
mod ctx;
mod facts;
mod hero;
mod media;
mod mods;
mod readme;
mod updates;

use freya::prelude::*;

use self::ctx::Ctx;
use super::common::{BackBar, NotFound, page_scroll};
use crate::{
    activity::hint_for_game,
    catalog::GameView,
    desktop::use_desktop_ui,
    effect::Effect,
    metrics::*,
    mod_games::takes_mods,
    nav::{use_nav, use_page_motion},
    prelude::*,
    shell::use_shell,
    store::{use_activity_of, use_community_of, use_games, use_moddable, use_mods, use_projects},
};

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
        let (games, projects, mods, moddable) = (use_games(), use_projects(), use_mods(), use_moddable());
        let activity = use_activity_of(self.id);
        let community = use_community_of(self.id);
        // Read the game's page on quiverlauncher.com each time the page opens (the host asks once however often this runs, and a
        // page already shown stays until the new one arrives), and once the game is linked if that happened after it opened.
        let on_effect = shell.on_effect.clone();
        let id = self.id;
        let linked = community.app.is_some();
        use_hook({
            let on_effect = on_effect.clone();
            move || {
                if linked {
                    on_effect.call(Effect::LoadCommunity(id));
                }
            }
        });
        if community.app.is_some() && community.page.is_none() {
            on_effect.call(Effect::LoadCommunity(id));
        }
        let view = GameView::resolve(self.id, &games, &projects, &mods);
        let Some(view) = view else {
            return NotFound { what: "game", id: self.id.to_string() }.into_element();
        };
        let failure = (view.game.status == AppStatus::Failed).then(|| hint_for_game(&activity, self.id));
        let takes_mods = takes_mods(self.id, &moddable);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0., |d| d.as_millis() as f64);
        let c = Ctx { t, env, view, nav, dialogs: ui.dialogs, on_effect: shell.on_effect.clone(), failure, ui, takes_mods, community, now };

        // After the banner has settled, the sections below it rise in one after another.
        let section = |index: usize, content: Option<Element>| {
            let (opacity, dy) = motion.rise(0.45 + 0.1 * index as f32, 1., 20.);
            content.map(|el| rect().width(Size::fill()).opacity(opacity).offset_y(dy).child(el).into_element())
        };

        let wide = env.class == LayoutClass::Wide;
        let main = [
            section(0, about::view(&c)),
            section(1, community::feedback(&c)),
            section(2, media::view(&c)),
            section(3, community::releases(&c).or_else(|| updates::view(&c))),
            section(4, mods::view(&c)),
            section(5, readme::view(&c)),
        ];
        let side = [
            section(1, facts::details(&c)),
            section(2, community::about(&c)),
            section(3, facts::requirements(&c)),
            section(4, facts::links(&c)),
        ];

        let body = if wide {
            rect()
                .horizontal()
                .content(Content::Flex)
                .spacing(SPACE_5)
                .width(Size::fill())
                .child(rect().vertical().spacing(SPACE_5).width(Size::flex(1.)).children(main.into_iter().flatten()))
                .child(rect().vertical().spacing(SPACE_5).width(Size::px(300.)).children(side.into_iter().flatten()))
        } else {
            let [about, feedback, media, updates, mods, readme] = main;
            let [details, site, requirements, links] = side;
            rect()
                .vertical()
                .spacing(SPACE_5)
                .width(Size::fill())
                .children([about, feedback, media, updates, details, site, requirements, mods, readme, links].into_iter().flatten())
        };

        let narrow = env.class != LayoutClass::Wide;
        page_scroll(
            &env,
            rect().vertical().spacing(SPACE_5).width(Size::fill()).child(BackBar {}).child(hero::view(&c, narrow)).child(body),
        )
    }
}
