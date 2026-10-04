//! The app root and the rules for switching between the desktop and Deck interfaces.
//!
//! * `model`: which interface is showing (pure, tested); `overrides`: environment switches for testing;
//! * `root`: the `Shell` component that wires them to the screen and the F9 / F10 keys.
mod model;
mod overrides;
mod root;

pub use model::{SIM_KEYBOARD_SHARE, ShellModel, keyboard_height, other, resolve};
pub use overrides::{DevOverrides, KeyboardStart};
pub use root::Shell;
