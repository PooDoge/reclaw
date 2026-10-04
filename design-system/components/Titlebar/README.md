The window's title bar when Reclaw draws its own frame (the default on Linux, where GNOME leaves decoration to the application). Rust: `reclaw_ui::window::Titlebar`; 36px tall (`TITLEBAR_H`, a Rust constant), buttons 46 wide.

* The drag area is the app name and the page being shown; dragging moves the window, a double press maximizes or restores. Resize bands along the edges and the rounded corners come from the toolkit's `BorderlessPlugin`.
* Minimize, maximize or restore, and close. Hover is `bg-raised`; close goes `danger` with `on-accent` text, like every desktop's.
* Desktop interface only: Deck mode fills the screen and has no use for window buttons. `RECLAW_WINDOW_FRAME=native` turns the custom frame off.
* Every button emits `Effect::Window(WindowCommand)`; nothing here calls the windowing library.

Verified on X11 (Xvfb) with `scripts/x11-smoke.sh`; **not verified on Wayland/GNOME**. Transparent windows depend on the compositor.
