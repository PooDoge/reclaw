use freya::prelude::*;

use super::ctx::Ctx;
use crate::{
    desktop::pages::common::{ModRow, heading},
    metrics::*,
    prelude::*,
};

/// How many mods the page lists before sending the user to the Mods tab.
const SHOWN: usize = 3;

/// Mods made for this game, and the way to all of them on the Mods tab. Shown for a game that takes
/// mods even before its sites have listed any; nothing for a game that does not.
pub(super) fn view(c: &Ctx) -> Option<Element> {
    if c.view.mods.is_empty() && !c.takes_mods {
        return None;
    }
    let rows = c
        .view
        .mods
        .iter()
        .take(SHOWN)
        .map(|m| ModRow { entry: m.clone(), key: DiffKey::None }.key(format!("{}/{}", m.provider.slug(), m.id)).into_element());
    let label = match c.view.mods.len() {
        0 => "Browse mods for this game".to_string(),
        n => format!("Browse all {n} mods for this game"),
    };
    Some(
        rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(heading(&c.t, "Mods for this game"))
            .children(rows)
            .child(ActionButton::new(ButtonVariant::Ghost).icon(IconName::Mods).label(label).on_press(c.show_mods()))
            .into_element(),
    )
}
