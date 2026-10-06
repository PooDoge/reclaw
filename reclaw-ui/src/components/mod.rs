//! Desktop-first building blocks: buttons, chips, rows, capsules, the nav and the hero. Each is a
//! single component with its own file; screens compose them. Deck mode has its own set in `deck`.
mod action_button;
mod art;
mod banner_art;
mod download_item;
mod failed_hint;
mod filter_chip;
mod game_capsule;
mod hero_header;
mod indicator;
mod install_dialog;
mod library_row;
mod nav;
mod remote_art;
mod search_field;
mod status_badge;
mod system_badge;
mod toggle_switch;

pub use action_button::{ActionButton, ButtonSize, ButtonVariant};
pub use art::ArtPlaceholder;
pub use banner_art::BannerArt;
pub use download_item::DownloadItem;
pub use failed_hint::FailedHint;
pub use filter_chip::FilterChip;
pub use game_capsule::GameCapsule;
pub use hero_header::HeroHeader;
pub use indicator::{IndicatorBadge, indicator_look, progress_strip};
pub use install_dialog::InstallDialog;
pub use library_row::LibraryRow;
pub use nav::{Nav, NavMode, NavTarget};
pub use remote_art::RemoteArt;
pub use search_field::SearchField;

pub use status_badge::{StatusBadge, status_tone};
pub use system_badge::SystemBadge;
pub use toggle_switch::ToggleSwitch;

use freya::prelude::*;

/// Provided by an interface that does its own focus handling (Deck mode moves focus with the pad,
/// the arrow keys and spatial navigation). Buttons under it do not join Freya's Tab order: a button
/// that held Freya focus would also fire on Enter, on top of the interface's own Confirm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManagedFocus;

pub type PressHandler = EventHandler<Event<PressEventData>>;

/// Tracks pointer hover on any element. Kept in one place so every custom component hovers alike.
pub(crate) fn hoverable<E: EventHandlersExt>(el: E, mut hovering: State<bool>) -> E {
    el.on_pointer_over(move |_| hovering.set_if_modified(true)).on_pointer_out(move |_| hovering.set_if_modified(false))
}

pub(crate) fn pointer_cursor<E: EffectExt>(el: E) -> E {
    el.cursor(CursorIcon::Pointer)
}

pub(crate) const CLEAR: Color = Color::from_argb(0, 0, 0, 0);
