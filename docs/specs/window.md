# The window, the monitors and the screen

- last-verified: 2026-10-04
- owner-paths: reclaw-ui/src/window/**, reclaw-ui/src/shell/services.rs, scripts/x11-smoke.sh, reclaw-ui/tests/ui/window_chrome.rs, packaging/**

How the window is framed, how Reclaw learns about the monitors, where the window opens, and how Deck mode uses the screen.

## The frame

By default the window has **no native decoration** and is transparent. Reclaw draws its own: a title bar in the desktop interface
(the page's name on a drag area, minimize, maximize or restore, close) and resize bands along the edges with rounded corners (the
toolkit's `BorderlessPlugin`). Deck mode has no title bar; it fills the screen. `RECLAW_WINDOW_FRAME=native` gives the window
manager's border instead, the way out if a compositor mishandles transparent undecorated windows.

`Frame` says who draws the border. `WindowHost { frame, attached }` is what the `Shell` is told about the window behind it;
`attached` is false in the headless tests, which then never ask the windowing library for anything (it panics for a window id
there). The window driver and the window commands do nothing unless `attached`.

## Asking for things

Components emit `Effect::Window(WindowCommand)` (minimize, toggle maximize, close, fullscreen on a monitor, windowed). The `Shell`
carries them out (`window::platform::run`) and also passes them to the host. **Close records the window and writes the settings
itself before closing**: the toolkit's close call does not run the host's close hook, which is where settings are normally flushed.

## Monitors

* Read when the window is created (so the first frame knows them), whenever it regains focus (the moment someone has plugged a
  monitor in), and when the scale changes. winit 0.30 has no monitor hot-plug event.
* Turned into the `DisplayEnvironment` the launch settings read (`window::monitors`): id (the connector name, made unique, or
  `monitor-N`), name, native size, refresh rate, primary.
* **Monitors are numbered left to right, then top to bottom** ("Monitor 1" is the leftmost). The same order picks the monitor in
  the Deck mode setting and the matching winit handle.
* The primary monitor is the system's answer where it gives one, else the monitor at the desktop's origin, else the first. **On
  Wayland winit cannot say which is primary, so this is a guess there.**
* The display server (Wayland, X11, gamescope, Windows, macOS) comes from the session's variables; gamescope is checked first
  because it sets Wayland variables too.

## Where the window opens

Size, place, maximized state and monitor are remembered (`window::geometry::remember`; the store saves them, debounced) when the
window is resized, changes scale, gains or loses focus, or is closed with its own button. A move alone raises no event, so a new
position is saved at the next of those. While the window is maximized or fullscreen only the flag is saved, so un-maximizing
returns to the old window.

On the next start `restore` decides: a saved size is made safe (whole pixels, within limits, brought down to the biggest monitor
if it is larger than all of them); a saved position is used **only where a program may place its windows (X11, Windows)** and only
if some part of the title bar would be on a connected monitor; Wayland compositors choose placement, so there only the size is used.

## Deck mode and the screen

Settings > Screen: *Fill the screen in Deck mode* (default on) and *Deck mode monitor* (Same as window, Monitor 1 to 4). Entering
Deck mode makes the window borderless-fullscreen on that monitor (no video mode change); leaving gives it back **only if Reclaw made
it fullscreen**, so a window the user made fullscreen by hand is left alone (`window::policy::on_mode`). A chosen monitor that is
not connected means the one the window is on. The *UI scale* row multiplies the system's scale factor.

## Verified, and not

`scripts/x11-smoke.sh` runs the real binary under Xvfb with openbox and checks, with xdotool: the title bar's three buttons, dragging,
the resize bands, Deck mode fullscreen and back, the window's app-id class, closing, and the size and place restored on the next start
(16 checks, all pass). **Not verified:** native Wayland / GNOME, transparency and rounded corners (no compositor in that rig), more
than one monitor (a virtual X server has one), high-DPI, `Platform.root_size` and the toolkit's resize bands on a scaled display,
Windows and macOS.

## Tests

Pure: `window/{monitors,geometry,policy,frame}` unit tests. Headless: `tests/ui/window_chrome.rs` (title bar buttons emit the right
effects, none in Deck mode, the Screen settings). Real: `scripts/x11-smoke.sh`.
