use freya::prelude::*;

use super::common::{capsule_grid, columns_for, empty_state, page_scroll, tab_header_with};
use crate::{
    catalog::{catalog_entries, platforms},
    desktop::use_desktop_ui,
    effect::Effect,
    metrics::*,
    nav::use_nav,
    prelude::*,
    shell::use_shell,
    store::{CatalogPhase, now_secs, use_catalog_status, use_games, use_projects, use_settings},
    systems::Sort,
};

/// Every known recompilation project, filtered by platform and the search box. A card opens the
/// project's page whether or not it is installed.
#[derive(PartialEq)]
pub struct CatalogPage {}

impl Component for CatalogPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let ui = use_desktop_ui();
        let nav = use_nav();
        let env = *ui.env.read();
        let (games, projects, settings, status, shell) = (use_games(), use_projects(), use_settings(), use_catalog_status(), use_shell());
        let mut platform = ui.platform;
        let current = *platform.read();

        let entries = catalog_entries(&games, &projects, current, &ui.search.read(), Sort::from_settings(&settings));
        let chip = |label: &'static str, count: u32, which: Option<_>| {
            FilterChip::new(label).count(count).selected(current == which).on_press(move |_| platform.set(which)).key(label)
        };
        let chips = rect()
            .horizontal()
            .content(Content::wrap_spacing(SPACE_2))
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(chip("All", projects.len() as u32, None))
            .children(platforms(&projects).into_iter().map(|(p, n)| chip(p.label(), n, Some(p)).into_element()));

        let body = if entries.is_empty() {
            {
                let say = crate::empty::catalog(&status, projects.len());
                empty_state(&t, say.title, say.text)
            }
        } else {
            capsule_grid(&entries, columns_for(&env), Some(ui.selected), nav)
        };
        let on_effect = shell.on_effect.clone();
        let refresh = ActionButton::new(ButtonVariant::Ghost)
            .icon(IconName::Refresh)
            .label("Refresh")
            .enabled(status.phase != CatalogPhase::Loading)
            .on_press(move |_| on_effect.call(Effect::RefreshCatalog))
            .into_element();
        page_scroll(
            &env,
            rect()
                .vertical()
                .spacing(SPACE_4)
                .width(Size::fill())
                .child(tab_header_with(&t, &env, "Catalog", status.line(projects.len(), now_secs()), ui.search, Some(refresh)))
                .child(chips)
                .child(body),
        )
    }
}
