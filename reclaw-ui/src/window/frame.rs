/// Who draws the window's border and title bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Frame {
    /// The window has no native decoration: Reclaw draws a title bar with the window buttons, and
    /// resize bands along the edges.
    Custom,
    /// The window manager draws them. The escape hatch for a compositor that mishandles
    /// transparent, undecorated windows.
    Native,
}

impl Frame {
    /// `RECLAW_WINDOW_FRAME=native` picks the window manager's decoration; anything else (or
    /// nothing) is the custom frame. `get` is injected so this is testable.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Self {
        match get("RECLAW_WINDOW_FRAME").map(|v| v.trim().to_lowercase()).as_deref() {
            Some("native" | "system" | "decorated") => Self::Native,
            _ => Self::Custom,
        }
    }
}

/// What stands behind the UI: how its window is framed, and whether there is a real window at all.
/// The `Shell` takes one; the window driver and the window commands do nothing unless `attached`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowHost {
    pub frame: Frame,
    /// A real window (made with `launch::launch_config`) is behind the UI. False in the headless
    /// tests and previews, which have no window to ask anything of.
    pub attached: bool,
}

impl WindowHost {
    /// No real window: the tests and anything else that renders without one. Native frame, so the
    /// desktop draws no title bar of its own.
    pub const DETACHED: Self = Self { frame: Frame::Native, attached: false };

    pub const fn attached(frame: Frame) -> Self {
        Self { frame, attached: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(value: Option<&str>) -> Frame {
        Frame::from_env(|_| value.map(str::to_string))
    }

    #[test]
    fn custom_unless_native_is_asked_for() {
        assert_eq!(frame(None), Frame::Custom);
        assert_eq!(frame(Some("custom")), Frame::Custom);
        assert_eq!(frame(Some("what")), Frame::Custom, "a typo must not change the window");
        assert_eq!(frame(Some(" Native ")), Frame::Native);
        assert_eq!(frame(Some("system")), Frame::Native);
    }
}
