//! Pieces more than one layout uses.
use freya::prelude::*;

use super::{ctx::Ctx, filter::Filter};
use crate::{
    desktop::{OpenPicker, press_point, press_verb},
    effect::Effect,
    metrics::*,
    nav::Route,
    prelude::*,
    settings::{KEY_LIBRARY_SORT, SettingValue, SettingsTarget},
    systems::Sort,
    typography::TypeStyle,
};

/// The status chips, and under them the two ways to browse by system: narrow to one, and sort.
pub(super) fn chips(c: &Ctx) -> Rect {
    rect().vertical().spacing(SPACE_2).width(Size::fill()).child(status_chips(c)).child(system_chips(c))
}

/// "System: All" and "Sort: Title". Each opens a list to choose from, where it was pressed.
fn system_chips(c: &Ctx) -> Rect {
    let (dialogs, mut system, on_effect) = (c.dialogs, c.system, c.on_effect.clone());
    let current = *system.read();
    let systems = c.systems.clone();
    let system_label = current.map_or("All systems", |p| p.label());
    let pick_system = move |e: Event<PressEventData>| {
        let mut labels = vec!["All systems".to_string()];
        labels.extend(systems.iter().map(|(p, n)| format!("{} ({n})", p.label())));
        let selected = current.and_then(|p| systems.iter().position(|(s, _)| *s == p)).map_or(0, |i| i + 1);
        let systems = systems.clone();
        let on_pick = EventHandler::new(move |i: usize| system.set(i.checked_sub(1).and_then(|i| systems.get(i)).map(|(p, _)| *p)));
        dialogs.pick(OpenPicker::new("System", labels, selected, press_point(&e, (120., 160.)), on_pick));
    };
    let sort = c.sort;
    let pick_sort = move |e: Event<PressEventData>| {
        let labels = Sort::ALL.iter().map(|s| s.label().to_string()).collect();
        let on_effect = on_effect.clone();
        let on_pick = EventHandler::new(move |i: usize| {
            for effect in Effect::setting(SettingsTarget::Global, KEY_LIBRARY_SORT, SettingValue::Choice(i)) {
                on_effect.call(effect);
            }
        });
        dialogs.pick(OpenPicker::new("Sort by", labels, sort.index(), press_point(&e, (120., 160.)), on_pick));
    };
    rect()
        .horizontal()
        .content(Content::wrap_spacing(SPACE_2))
        .spacing(SPACE_2)
        .width(Size::fill())
        .child(FilterChip::new(system_label).selected(current.is_some()).on_press(pick_system))
        .child(FilterChip::new(format!("Sort: {}", sort.label())).on_press(pick_sort))
}

fn status_chips(c: &Ctx) -> Rect {
    let (mut filter, all, installed, updates) = (c.filter, c.games.len() as u32, c.installed, c.updates);
    let favorites = c.games.iter().filter(|g| g.is_favorite()).count() as u32;
    let chip = move |label: &'static str, count: u32, which: Filter| {
        FilterChip::new(label).count(count).selected(filter() == which).on_press(move |_| filter.set(which))
    };
    rect()
        .horizontal()
        .content(Content::wrap_spacing(SPACE_2))
        .spacing(SPACE_2)
        .width(Size::fill())
        .child(chip("All", all, Filter::All))
        .child(chip("Installed", installed, Filter::Installed))
        .child(chip("Updates", updates, Filter::Updates))
        .child(chip("Favorites", favorites, Filter::Favorites))
}

/// The selected game's hero. The verb button acts, Details and the banner open the game's page, and
/// Manage opens the Options menu where it was pressed.
pub(super) fn hero(c: &Ctx, game: GameEntry, narrow: bool) -> HeroHeader {
    let (dialogs, on_effect, nav) = (c.dialogs, c.on_effect.clone(), c.nav);
    let id = game.id;
    let (verb_game, menu_game, folder_effect) = (game.clone(), game.clone(), on_effect.clone());
    let verb_effect = on_effect;
    HeroHeader::new(game)
        .narrow(narrow)
        .density(c.env.density)
        .on_primary(move |_| press_verb(&verb_game, dialogs, &verb_effect))
        .on_open(move |_| nav.open(Route::Game { id }))
        .on_open_folder(move |_| folder_effect.call(Effect::OpenFolder(id)))
        .on_manage(move |e: Event<PressEventData>| dialogs.manage(&menu_game, press_point(&e, (320., 160.))))
}

/// The jobs still running or failed, in the main column. Finished ones are in the Updates section and the Downloads tab.
pub(super) fn downloads_list(c: &Ctx) -> Option<Rect> {
    let t = c.t;
    let open: Vec<_> =
        c.activity.queue().into_iter().filter(|a| !matches!(a.outcome, crate::activity::Outcome::Finished)).cloned().collect();
    if open.is_empty() {
        return None;
    }
    Some(rect().vertical().spacing(SPACE_2).width(Size::fill()).child(TypeStyle::Eyebrow.text("Downloads", t.ink_subtle)).children(
        open.into_iter().map(|d| {
            let (id, running, on_effect) = (d.id, d.is_running(), c.on_effect.clone());
            DownloadItem::new(d)
                .on_cancel(move |_| on_effect.call(if running { Effect::CancelActivity(id) } else { Effect::DismissActivity(id) }))
                .key(id)
                .into_element()
        }),
    ))
}

/// Capsule grid in fixed-column rows of fluid capsules, so the last row keeps its column width.
/// Pressing a capsule opens that game's page.
pub(super) fn grid(c: &Ctx, columns: usize) -> Rect {
    let (mut selected, nav) = (c.selected, c.nav);
    rect().vertical().spacing(SPACE_3).width(Size::fill()).children(c.visible.chunks(columns).enumerate().map(|(row, chunk)| {
        let mut line = rect().horizontal().content(Content::Flex).spacing(SPACE_3).width(Size::fill());
        for game in chunk {
            let id = game.id;
            line = line.child(
                rect().width(Size::flex(1.)).child(
                    GameCapsule::new(game.clone())
                        .fluid(true)
                        .selected(Some(id) == selected())
                        .on_press(move |_| {
                            selected.set(Some(id));
                            nav.open(Route::Game { id });
                        })
                        .key(id),
                ),
            );
        }
        for _ in chunk.len()..columns {
            line = line.child(rect().width(Size::flex(1.)));
        }
        line.key(row).into_element()
    }))
}

/// The switch to Deck mode, as a button.
pub(super) fn deck_button(c: &Ctx) -> ActionButton {
    let toggle = c.toggle_deck.clone();
    ActionButton::new(ButtonVariant::Ghost)
        .icon(IconName::Gamepad)
        .label("Deck mode")
        .size(ButtonSize::for_density(c.env.density, false))
        .on_press(move |_| toggle.call(()))
}

/// The search field, with the Deck mode button beside it on layouts that have no top bar.
pub(super) fn search_row(c: &Ctx) -> Rect {
    rect()
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(SPACE_2)
        .width(Size::fill())
        .child(rect().width(Size::flex(1.)).child(SearchField::new(c.search).density(c.env.density)))
        .child(deck_button(c))
}
