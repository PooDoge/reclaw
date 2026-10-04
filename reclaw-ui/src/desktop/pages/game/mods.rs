use freya::prelude::*;

use super::ctx::Ctx;
use crate::{
    desktop::pages::common::{ModRow, heading},
    metrics::*,
    nav::Route,
    prelude::*,
};

/// How many mods the page lists before sending the user to the Mods tab.
const SHOWN: usize = 3;

/// Mods made for this game. Nothing when there are none.
pub(super) fn view(c: &Ctx) -> Option<Element> {
    if c.view.mods.is_empty() {
        return None;
    }
    let nav = c.nav;
    let rows = c
        .view
        .mods
        .iter()
        .take(SHOWN)
        .map(|m| ModRow { entry: m.clone(), key: DiffKey::None }.key(format!("{}/{}", m.provider.slug(), m.id)).into_element());
    Some(
        rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(heading(&c.t, "Mods for this game"))
            .children(rows)
            .child(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Mods)
                    .label("Browse all mods")
                    .on_press(move |_| nav.open(Route::Mods {})),
            )
            .into_element(),
    )
}
