use freya::prelude::*;

use super::common::{capsule_grid_with, columns_for, empty_state, no_results, page_scroll, results_bar, tab_header_with};
use crate::{
    catalog::platforms,
    catalog_browse::{CatalogView, browse, card_notes},
    desktop::use_desktop_ui,
    effect::Effect,
    metrics::*,
    nav::use_nav,
    prelude::*,
    search::SearchScope,
    shell::use_shell,
    store::{CatalogPhase, now_secs, use_catalog_status, use_games, use_projects, use_settings},
    typography::TypeStyle,
};

/// Every app the catalog offers (quiverlauncher.com's, then what only the frozen lists have), sorted and narrowed as the Catalog
/// settings say, by console chip and by the search box. A card says how the app runs for players and opens its page whether or
/// not it is installed.
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

        let query = ui.query(SearchScope::Catalog);
        let shown = browse(&games, &projects, current, &query, &CatalogView::from_settings(&settings));
        let (entries, hidden) = (shown.entries, shown.hidden_in_library);
        let notes = card_notes(&projects, now_secs());
        let on_clear = EventHandler::new(move |()| ui.clear_search(SearchScope::Catalog));
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

        let body = if entries.is_empty() && !query.is_empty() && !projects.is_empty() {
            no_results(&t, SearchScope::Catalog, &query, "Try other words, or another platform.", on_clear.clone())
        } else if entries.is_empty() && hidden > 0 && current.is_none() {
            empty_state(&t, "Everything here is in your library", "Turn off Hide apps in my library in Settings, Catalog to list them too.")
        } else if entries.is_empty() && !projects.is_empty() {
            empty_state(
                &t,
                "Nothing matches these filters",
                "Settings, Catalog says which apps are shown: try All platforms or every project type.",
            )
        } else if entries.is_empty() {
            let say = crate::empty::catalog(&status, projects.len());
            empty_state(&t, say.title, say.text)
        } else {
            capsule_grid_with(&entries, &notes, columns_for(&env), Some(ui.selected), nav)
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
                .child(tab_header_with(&t, &env, "Catalog", status.line(projects.len(), now_secs()), Some(refresh)))
                .child(chips)
                .maybe_child((hidden > 0 && !entries.is_empty()).then(|| {
                    let what = if hidden == 1 { "1 app in your library is hidden" } else { "apps in your library are hidden" };
                    let text = if hidden == 1 { what.to_string() } else { format!("{hidden} {what}") };
                    TypeStyle::Meta.text(format!("{text}. Settings, Catalog shows them again."), t.ink_subtle)
                }))
                .maybe_child(
                    (!query.is_empty() && !entries.is_empty())
                        .then(|| results_bar(&t, SearchScope::Catalog, &query, entries.len(), false, on_clear)),
                )
                .child(body),
        )
    }
}
