//! The desktop interface: a persistent frame (navigation, status bar, dialogs) around the routed
//! pages. Pages are drawn by `pages::*` (the route components) through `desktop::pages`.
//!
//! * `frame`: `DesktopFrame`, what stays while pages change; `ui`: the state it shares with every desktop page
//! * `actions`: what the common buttons do; `dialogs`: install, Manage and uninstall, opened by any page
//! * `pages`: one folder or file per page
mod actions;
mod dialogs;
mod frame;
pub mod pages;
mod ui;

pub use actions::{press_point, press_verb};
pub use dialogs::{GameDialogs, GameDialogsLayer, OpenPicker};
pub use frame::DesktopFrame;
pub use ui::{DesktopEnv, DesktopUi, use_desktop_ui};
