/// What the UI asks of the window. Part of the `Effect` vocabulary: components emit it and the
/// `Shell` carries it out (see `platform::run`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WindowCommand {
    Minimize,
    ToggleMaximize,
    /// Save what is unsaved, then close. Closing through the window manager's own button runs the
    /// host's close hook; this command runs the same save itself, because the toolkit's close call
    /// skips the hook.
    Close,
    /// Fill a monitor without changing its video mode (borderless fullscreen). `monitor` is its
    /// zero-based place in the left-to-right order; `None` is whichever the window is on.
    Fullscreen {
        monitor: Option<usize>,
    },
    /// Leave fullscreen.
    Windowed,
    /// The pointer went down on a resize band: hand the rest of the drag to the window manager.
    BeginResize(super::resize::Edge),
}
