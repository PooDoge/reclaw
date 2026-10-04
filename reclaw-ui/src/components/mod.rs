mod action_button;
mod art;
mod download_item;
mod filter_chip;
mod game_capsule;
mod hero_header;
mod install_dialog;
mod library_row;
mod nav;
mod search_field;
mod status_badge;
mod toggle_switch;

pub use action_button::{ActionButton, ButtonSize, ButtonVariant};
pub use art::ArtPlaceholder;
pub use download_item::DownloadItem;
pub use filter_chip::FilterChip;
pub use game_capsule::GameCapsule;
pub use hero_header::HeroHeader;
pub use install_dialog::InstallDialog;
pub use library_row::LibraryRow;
pub use nav::{Nav, NavItem, NavMode};
pub use search_field::SearchField;

pub use status_badge::{StatusBadge, status_tone};
pub use toggle_switch::ToggleSwitch;

use freya::prelude::*;

pub type PressHandler = EventHandler<Event<PressEventData>>;

/// Tracks pointer hover on any element. Kept in one place so every custom component hovers alike.
pub(crate) fn hoverable<E: EventHandlersExt>(el: E, mut hovering: State<bool>) -> E {
    el.on_pointer_over(move |_| hovering.set_if_modified(true))
        .on_pointer_out(move |_| hovering.set_if_modified(false))
}

pub(crate) fn pointer_cursor<E: EffectExt>(el: E) -> E {
    el.cursor(CursorIcon::Pointer)
}

pub(crate) const CLEAR: Color = Color::from_argb(0, 0, 0, 0);
