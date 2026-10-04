//! The app root and the rules for switching between the desktop and Deck interfaces.
//!
//! * `model`: which interface is showing (pure, tested); `overrides`: environment switches for testing
//! * `root`: the `Shell` component, which mounts the router and handles F9 / F10
//! * `ctx`: what the shell hands down to the layout and the pages
mod ctx;
mod model;
mod overrides;
mod root;

pub use ctx::{ShellCtx, use_shell};
pub use model::{SIM_KEYBOARD_SHARE, ShellModel, keyboard_height, other, resolve};
pub use overrides::{DevOverrides, KeyboardStart, MotionOverride};
pub use root::Shell;
