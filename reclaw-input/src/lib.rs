//! Gamepad input for Reclaw: raw events in, semantic UI actions out.
//!
//! Everything except [`backend`] is pure and hardware-free, so it is unit-tested. The mapper owns
//! the rules that matter for a console-style UI: stick hysteresis, hold-to-repeat, and *input
//! ownership* (while an app is running the launcher stops navigating; only Guide gets through).
mod action;
mod controller;
mod env;
mod mapper;
mod spatial;

#[cfg(feature = "gilrs-backend")]
pub mod backend;

pub use action::{Action, ActionMap, Axis, Button, Direction, RawEvent};
pub use controller::{ControllerInfo, ControllerKind, GlyphFace, PowerState};
pub use env::{Environment, GuideOwner, UiMode, detect_environment};
pub use mapper::{InputMapper, InputOwner, MapperConfig};
pub use spatial::{FocusId, FocusNode, Rect, next_focus};
