//! Navigation: which page is showing, how the user got there, and how one page gives way to the next.
//!
//! * `route`: every page as one `freya-router` enum; `meta`: what is known about a route without drawing it
//! * `section`: the top-level tabs; `recents`: the pages visited lately; `titles`: what to call a page in a list
//! * `transition`: styles, per-interface settings and frame maths (all pure)
//!
//! * `handle`: `Nav`, the one way code changes page; `stage`: plays transitions between pages (`stage_state` is its pure core)
//! * `motion`: where a page is in its entrance, for pages that choreograph it; `input`: mouse side buttons, Alt+arrows, Escape
//! * `layers`: dialogs and menus over a page, which Back closes before it leaves the page
//!
//! The pure modules need no window to test; the rest is covered by the integration tests.
mod handle;
mod input;
mod layers;
mod meta;
mod motion;
mod recents;
mod route;
mod section;
mod stage;
mod stage_state;
mod titles;
pub mod transition;

pub use handle::{Nav, use_nav};
pub use input::NavInputExt;
pub use layers::{LayerId, Layers};
pub use meta::PageKind;
pub use motion::{PageMotion, use_page_motion};
pub use recents::Recents;
pub use route::Route;
pub use section::Section;
pub use stage::RouteStage;
pub use stage_state::StageState;
pub use titles::title;
