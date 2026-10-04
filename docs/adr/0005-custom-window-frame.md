# 0005 Reclaw draws its own window frame

- status: accepted
- freya: claims about the toolkit checked against 0.5.0-rc.8 (tag v0.5.0-rc.8, commit af55a77) on 2026-10-04
- date: 2026-10-04
- spec: ../specs/window.md

## Context

The target is Bazzite on GNOME and Wayland, plus SteamOS-style gaming sessions. GNOME leaves window decorations to the application;
a Steam-like launcher also wants one look on every desktop, a title bar that can carry its own controls, and a Deck mode that fills
the screen with no chrome.

## Decision

Windows have no native decoration and are transparent; the toolkit's `BorderlessPlugin` supplies resize bands and rounded corners,
and a title bar of our own (drag area, minimize, maximize, close) shows in the desktop interface. `RECLAW_WINDOW_FRAME=native` turns
it all off. Every request to the window is an `Effect` the `Shell` carries out; the `Shell` is told whether a real window stands behind it.

## Rejected

* **Native decorations everywhere.** On GNOME they are drawn by the toolkit's fallback, differ from KDE and Windows, and cannot hold
  our controls. Kept as the escape hatch.
* **Native decoration with a custom bar inside it.** Two title bars.
* **Calling the windowing library from components.** Against the rule that the UI asks only through `Effect`s, and the headless
  tests would need a window.
* **The borderless plugin's `use_maximized` hook** (`freya-borderless-plugin`). Its side effect calls `Platform::window_id()`, which
  reads a root context and panics when there is none, as in the headless tests; a guarded copy is used.
* **The toolkit's close call for the Close button.** `Platform::close_window` removes the window and tells plugins, but only the
  window manager's close request (`CloseRequested`) runs the `on_close` hook, so settings flushed there would be lost; the command
  flushes first itself.

## Consequences

Transparency depends on the compositor: if corners are black or the window is invisible, the escape hatch is the answer. The frame
adds 36 px to the desktop. The behaviour is verified on X11 only (`scripts/x11-smoke.sh`); Wayland/GNOME is unverified.
