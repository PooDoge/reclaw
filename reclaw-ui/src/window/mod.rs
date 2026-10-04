//! The app window: how it is framed, which monitors exist, where it opens, and how the UI asks the
//! windowing system for things. Only `platform`, `driver`, `titlebar` and `launch` touch Freya or
//! winit; the rest is pure and tested.
//!
//! * `command`: [`WindowCommand`], the window's share of the effect vocabulary
//! * `monitors`: raw monitor facts to the `DisplayEnvironment` the launch settings use, numbering,
//!   display server detection
//! * `geometry`: restoring a saved size and position safely, and what is worth saving
//! * `policy`: what the settings say about the window (Deck fullscreen, which monitor, UI scale)
//! * `frame`: custom (borderless, our own title bar) or native decorations
//! * `resize`: where the resize bands go along the edges, in layout units (pure); `bands`: the component
//! * `platform`: the winit calls (read monitors, read the window, run a command)
//! * `driver`: `use_window_driver`, the hook the `Shell` runs to keep all of this in step
//! * `titlebar`: the drag area and the minimize, maximize and close buttons
//! * `launch`: the window and plugin configuration a host launches with
//!
//! Monitors are numbered left to right, then top to bottom ("Monitor 1" is the leftmost), the way
//! people count them. Wayland gives no way to ask which one is the primary.
mod bands;
mod command;
mod driver;
mod frame;
mod geometry;
pub mod launch;
mod monitors;
mod platform;
mod policy;
pub mod resize;
mod titlebar;

pub use bands::ResizeBands;
pub use command::WindowCommand;
pub use driver::{run_command, use_window_driver};
pub use frame::{Frame, WindowHost};
pub use geometry::{DEFAULT_SIZE, MIN_SIZE, Restore, Snapshot, remember, restore, sane_size};
pub use monitors::{RawMonitor, arrange, detect_server, environment, monitor_number, order_key};
pub use policy::{DeckDisplay, on_mode, ui_scale};
pub use resize::Edge;
pub use titlebar::{TITLEBAR_H, Titlebar};
