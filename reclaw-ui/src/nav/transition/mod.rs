//! How pages give way to each other. Everything here is pure data and maths; `nav::stage` plays it.
//!
//! * `style`: directions, styles, intensity, easing
//! * `config`: per-interface settings and `resolve`, which decides one transition
//! * `frame`: how each page is drawn at a moment of a transition
mod config;
mod frame;
mod style;

pub use config::{ModeProfile, Spec, TransitionConfig, resolve};
pub use frame::{Frame, Role, frame};
pub use style::{Direction, Easing, Intensity, TransitionStyle};
