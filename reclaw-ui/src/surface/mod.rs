//! Surfaces: dialogs, full-screen pages and menus, and the rules for which one to use where.
mod dialog;
pub mod menu;
mod modal_menu;
mod page;
pub mod presentation;
pub mod reveal;
mod rows;

pub use dialog::{Dialog, DialogAction};
pub use menu::{EntryKind, Level, MenuEntry, MenuState, Outcome};
pub use modal_menu::{MenuLevelView, MenuPlacement, MenuRowView, ModalMenu};
pub use page::{FullScreenPage, RevealScroll};
pub use presentation::{Presentation, SurfaceContext, SurfaceKind, TOUCH_POPUP_MIN_HEIGHT, presentation};
pub use reveal::{scroll_to_reveal, visible_height};
pub use rows::{RowControl, SettingRow, row_height};
