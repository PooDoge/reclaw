use freya::prelude::*;

use super::ctx::Ctx;
use crate::{
    desktop::{press_failure, press_point, press_verb},
    effect::Effect,
    nav::Route,
    prelude::*,
};

/// The banner, the title, and the one row of actions: the install-or-play verb, favorite, the
/// game's settings, its folder and the Manage menu. A Failed badge shows why on hover and opens the log.
pub(super) fn view(c: &Ctx, narrow: bool) -> Element {
    let game = c.view.game.clone();
    let id = game.id;
    let (nav, dialogs) = (c.nav, c.dialogs);
    let (verb_game, menu_game) = (game.clone(), game.clone());
    let (verb_effect, favorite_effect, folder_effect, failure_effect) =
        (c.on_effect.clone(), c.on_effect.clone(), c.on_effect.clone(), c.on_effect.clone());
    HeroHeader::new(game.clone())
        .narrow(narrow)
        .density(c.env.density)
        .on_primary(move |_| press_verb(&verb_game, dialogs, &verb_effect))
        .on_favorite(game.is_favorite(), move |_| favorite_effect.call(Effect::ToggleFavorite(id)))
        .on_settings(move |_| nav.open(Route::GameSettings { id }))
        .on_open_folder(move |_| folder_effect.call(Effect::OpenFolder(id)))
        .on_manage(move |e: Event<PressEventData>| dialogs.manage(&menu_game, press_point(&e, (320., 160.))))
        .map(c.failure.clone(), |hero, hint| {
            let text = hint.text.clone();
            hero.failure(text, move |_| press_failure(&hint, dialogs, &failure_effect))
        })
        .into_element()
}
