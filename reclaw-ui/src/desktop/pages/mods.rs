use freya::prelude::*;

use super::common::{ModRow, empty_state, page_scroll, tab_header_with};
use crate::{
    catalog::mods_matching,
    desktop::{OpenPicker, press_point, use_desktop_ui},
    effect::Effect,
    metrics::*,
    mod_games::{chosen, mod_games, picked, picker_index, picker_labels},
    model::ModProvider,
    prelude::*,
    shell::use_shell,
    store::{use_games, use_moddable, use_mods},
};

/// Mods from the sites Reclaw knows, narrowed to one installed game (or all), a provider and the
/// search box. A row opens the mod's page; its button installs or removes without opening it.
#[derive(PartialEq)]
pub struct ModsPage {}

impl Component for ModsPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let ui = use_desktop_ui();
        let env = *ui.env.read();
        let (mods, games, moddable, shell) = (use_mods(), use_games(), use_moddable(), use_shell());
        let (mut provider, mut mod_game, dialogs) = (ui.provider, ui.mod_game, ui.dialogs);
        let current = *provider.read();

        let choices = mod_games(&games, &moddable, &mods);
        let game = chosen(*mod_game.read(), &choices).cloned();
        let game_id = game.as_ref().map(|g| g.id);
        // The chips count within the chosen game, so a count always says what pressing it shows.
        let in_game: Vec<ModEntry> = mods_matching(&mods, game_id, None, "");
        let shown = mods_matching(&in_game, None, current, &ui.search.read());

        let pick_game = {
            let choices = choices.clone();
            move |e: Event<PressEventData>| {
                let choices = choices.clone();
                let labels = picker_labels(&choices);
                let selected = picker_index(game_id, &choices);
                let on_pick = EventHandler::new(move |i: usize| mod_game.set(picked(i, &choices)));
                dialogs.pick(OpenPicker::new("Game", labels, selected, press_point(&e, (120., 160.)), on_pick));
            }
        };
        let game_label = game.as_ref().map_or_else(|| "All games".to_string(), |g| g.title.clone());
        let game_chip = FilterChip::new(game_label).selected(game.is_some()).on_press(pick_game).key("game");

        let chip = |label: &'static str, count: u32, which: Option<ModProvider>| {
            FilterChip::new(label).count(count).selected(current == which).on_press(move |_| provider.set(which)).key(label)
        };
        let count_for = |p: ModProvider| in_game.iter().filter(|m| m.provider == p).count() as u32;
        let chips = rect()
            .horizontal()
            .content(Content::wrap_spacing(SPACE_2))
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(game_chip)
            .child(chip("All sites", in_game.len() as u32, None))
            .child(chip(ModProvider::Thunderstore.label(), count_for(ModProvider::Thunderstore), Some(ModProvider::Thunderstore)))
            .child(chip(ModProvider::GameBanana.label(), count_for(ModProvider::GameBanana), Some(ModProvider::GameBanana)));

        let on_effect = shell.on_effect.clone();
        let refresh = ActionButton::new(ButtonVariant::Ghost)
            .icon(IconName::Refresh)
            .label("Refresh")
            .on_press(move |_| on_effect.call(Effect::RefreshMods))
            .into_element();

        let list: Element = if shown.is_empty() {
            let say = crate::empty::mods(in_game.len(), game.is_some());
            empty_state(&t, say.title, say.text).into_element()
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
        let count = |n: usize| if n == 1 { "1 mod".to_string() } else { format!("{n} mods") };
        let summary = match &game {
            Some(g) => format!("{} for {}", count(in_game.len()), g.title),
            None => format!("{} for your games", count(mods.len())),
        };
        page_scroll(
            &env,
            rect()
                .vertical()
                .spacing(SPACE_4)
                .width(Size::fill())
                .child(tab_header_with(&t, &env, "Mods", summary, ui.search, Some(refresh)))
                .child(chips)
                .child(list),
        )
    }
}
