//! Pieces more than one layout uses.
use freya::prelude::*;

use super::{ctx::Ctx, filter::Filter};
use crate::{
    app_menu::{MenuAction, options_menu},
    components::InstallDialog,
    metrics::*,
    prelude::*,
    surface::{Dialog, DialogAction, MenuLevelView, MenuPlacement, ModalMenu, Outcome, SurfaceKind},
    typography::TypeStyle,
};

pub(super) fn chips(c: &Ctx) -> Rect {
    let (mut filter, all, installed, updates) = (c.filter, c.games.len() as u32, c.installed, c.updates);
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
}

/// The selected game's hero. Install opens the dialog; Manage opens the Options menu where it was pressed.
pub(super) fn hero(c: &Ctx, game: GameEntry, narrow: bool) -> HeroHeader {
    let needs_install = matches!(game.status, AppStatus::Available | AppStatus::NeedsFile);
    let (mut dialog_open, mut manage) = (c.dialog_open, c.manage);
    let menu = options_menu(&game, false);
    HeroHeader::new(game)
        .narrow(narrow)
        .density(c.density)
        .on_primary(move |_| {
            if needs_install {
                dialog_open.set(true);
            }
        })
        .on_manage(move |e: Event<PressEventData>| {
            let at = match &*e {
                PressEventData::Mouse(m) => (m.global_location.x as f32, m.global_location.y as f32),
                PressEventData::Touch(t) => (t.global_location.x as f32, t.global_location.y as f32),
                // Enter on the focused button: no pointer, so open near the top left of the content.
                PressEventData::Keyboard(_) => (320., 160.),
            };
            manage.set(Some((menu.clone(), at)));
        })
}

pub(super) fn downloads_list(c: &Ctx) -> Rect {
    let t = c.t;
    rect()
        .vertical()
        .spacing(SPACE_2)
        .width(Size::fill())
        .child(TypeStyle::Eyebrow.text("Downloads", t.ink_subtle))
        .children(c.downloads.iter().cloned().map(|d| DownloadItem::new(d).into_element()))
}

/// Capsule grid in fixed-column rows of fluid capsules, so the last row keeps its column width.
pub(super) fn grid(c: &Ctx, columns: usize) -> Rect {
    let (mut selected, mut page_open) = (c.selected, c.page_open);
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
                            page_open.set(true);
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

/// The install form, adaptive: a popup on desktop, a full-screen page on phones and handhelds.
pub(super) fn install_dialog(c: &Ctx) -> InstallDialog {
    let title = c.current.as_ref().map(|g| g.title.to_string()).unwrap_or_default();
    let (mut dialog_open, mut game_file) = (c.dialog_open, c.form.game_file);
    InstallDialog::new((c.dialog_open)(), &title, c.form.location, c.form.game_file, c.form.shortcut, c.form.prerelease)
        .surface(c.surface())
        .window(c.window)
        .keyboard_inset(c.keyboard_inset)
        // The real app opens a native file picker here; the gallery fills in a sample path.
        .on_choose_file(move |_| game_file.set(Some("~/Games/starfall64.z64".to_string())))
        .on_cancel(move |_| dialog_open.set(false))
        .on_confirm(move |_| dialog_open.set(false))
}

/// The Manage menu, when open: anchored where it was pressed on a pointer, centered otherwise.
pub(super) fn manage_menu(c: &Ctx) -> Option<Element> {
    let (menu, at) = c.manage.read().clone()?;
    let placement = match crate::surface::presentation(crate::surface::SurfaceKind::Menu, c.surface()) {
        crate::surface::Presentation::Anchored => MenuPlacement::Anchored { x: at.0, y: at.1 },
        _ => MenuPlacement::Centered,
    };
    let (mut state, mut confirm, game) = (c.manage, c.confirm, c.current.as_ref().map(|g| g.id));
    let on_action = c.on_action.clone();
    let dismiss = EventHandler::new(move |()| state.set(None));
    let pick = EventHandler::new(move |(level, index): (usize, usize)| {
        let Some((mut menu, at)) = state.peek().clone() else { return };
        match menu.pick(level, index) {
            Outcome::Moved => state.set(Some((menu, at))),
            Outcome::Chose(action) => {
                state.set(None);
                let Some(id) = game else { return };
                if action == MenuAction::Uninstall {
                    // Destructive: ask first, like Deck mode does.
                    confirm.set(Some(id));
                } else if let Some(h) = &on_action {
                    h.call((id, action));
                }
            }
            Outcome::Closed => state.set(None),
            Outcome::None => {}
        }
    });
    Some(ModalMenu::new(MenuLevelView::from_state(&menu), placement, c.window, c.density, pick, dismiss).into_element())
}

/// The switch to Deck mode, when the host offers one.
pub(super) fn deck_button(c: &Ctx) -> Option<ActionButton> {
    let on_deck = c.on_deck_mode.clone()?;
    Some(
        ActionButton::new(ButtonVariant::Ghost)
            .icon(IconName::Gamepad)
            .label("Deck mode")
            .size(ButtonSize::for_density(c.density, false))
            .on_press(move |_| on_deck.call(())),
    )
}

/// The search field, with the Deck mode button beside it on layouts that have no top bar.
pub(super) fn search_row(c: &Ctx) -> Rect {
    rect()
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(SPACE_2)
        .width(Size::fill())
        .child(rect().width(Size::flex(1.)).child(SearchField::new(c.search).density(c.density)))
        .maybe_child(deck_button(c))
}

/// "Uninstall X?", a small confirmation: a popup on desktop, a centered card on touch and Deck.
pub(super) fn uninstall_confirm(c: &Ctx) -> Option<Element> {
    let id = (c.confirm)()?;
    let game = c.games.iter().find(|g| g.id == id)?;
    let t = c.t;
    let (mut confirm, on_action) = (c.confirm, c.on_action.clone());
    let cancel = EventHandler::new(move |()| confirm.set(None));
    let accept = EventHandler::new(move |()| {
        confirm.set(None);
        if let Some(h) = &on_action {
            h.call((id, MenuAction::Uninstall));
        }
    });
    let body = TypeStyle::Body.text("Removes it from this device. Your own game file is never touched.", t.ink_muted);
    let actions = vec![
        DialogAction::new("Cancel", ButtonVariant::Ghost, cancel.clone()),
        DialogAction::new("Uninstall", ButtonVariant::Danger, accept),
    ];
    Some(
        Dialog::new(SurfaceKind::Confirm, c.surface(), c.window, format!("Uninstall {}?", game.title), body, actions, cancel)
            .into_element(),
    )
}
