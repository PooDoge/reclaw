use freya::prelude::*;

use super::common::{ModRow, empty_state, page_scroll, tab_header};
use crate::{catalog::mods_matching, desktop::use_desktop_ui, metrics::*, model::ModProvider, prelude::*, store::use_mods};

/// Mods from the sites Reclaw knows, filtered by provider and the search box. A row opens the
/// mod's page; its button installs or removes without opening it.
#[derive(PartialEq)]
pub struct ModsPage {}

impl Component for ModsPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let ui = use_desktop_ui();
        let env = *ui.env.read();
        let mods = use_mods();
        let mut provider = ui.provider;
        let current = *provider.read();

        let shown = mods_matching(&mods, current, &ui.search.read());
        let chip = |label: &'static str, count: u32, which: Option<ModProvider>| {
            FilterChip::new(label).count(count).selected(current == which).on_press(move |_| provider.set(which)).key(label)
        };
        let count_for = |p: ModProvider| mods.iter().filter(|m| m.provider == p).count() as u32;
        let chips = rect()
            .horizontal()
            .content(Content::wrap_spacing(SPACE_2))
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(chip("All", mods.len() as u32, None))
            .child(chip(ModProvider::Thunderstore.label(), count_for(ModProvider::Thunderstore), Some(ModProvider::Thunderstore)))
            .child(chip(ModProvider::GameBanana.label(), count_for(ModProvider::GameBanana), Some(ModProvider::GameBanana)));

        let list: Element = if shown.is_empty() {
            {
                let say = crate::empty::mods(!mods.is_empty());
                empty_state(&t, say.title, say.text).into_element()
            }
        } else {
            rect()
                .vertical()
                .spacing(SPACE_2)
                .width(Size::fill())
                .children(shown.into_iter().map(|m| {
                    let key = format!("{}/{}", m.provider.slug(), m.id);
                    ModRow { entry: m, key: DiffKey::None }.key(key).into_element()
                }))
                .into_element()
        };
        page_scroll(
            &env,
            rect()
                .vertical()
                .spacing(SPACE_4)
                .width(Size::fill())
                .child(tab_header(&t, &env, "Mods", format!("{} mods for your games", mods.len()), ui.search))
                .child(chips)
                .child(list),
        )
    }
}
