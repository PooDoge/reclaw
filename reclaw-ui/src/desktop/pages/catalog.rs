use freya::prelude::*;

use super::common::{capsule_grid, columns_for, empty_state, page_scroll, tab_header};
use crate::{
    catalog::{catalog_entries, platforms},
    desktop::use_desktop_ui,
    metrics::*,
    prelude::*,
    store::{use_games, use_projects},
};

/// Every known recompilation project, filtered by platform and the search box. A card opens the
/// project's page whether or not it is installed.
#[derive(PartialEq)]
pub struct CatalogPage {}

impl Component for CatalogPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let ui = use_desktop_ui();
        let env = *ui.env.read();
        let (games, projects) = (use_games(), use_projects());
        let mut platform = ui.platform;
        let current = *platform.read();

        let entries = catalog_entries(&games, &projects, current, &ui.search.read());
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
            empty_state(&t, "No projects match", "Try another platform or a different search.")
        } else {
            capsule_grid(&entries, columns_for(&env), Some(ui.selected))
        };
        page_scroll(
            &env,
            rect()
                .vertical()
                .spacing(SPACE_4)
                .width(Size::fill())
                .child(tab_header(&t, &env, "Catalog", format!("{} recompilation projects", projects.len()), ui.search))
                .child(chips)
                .child(body),
        )
    }
}
