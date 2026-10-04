# 0008 Reclaw places its own resize bands

- status: accepted
- freya: claims about the toolkit checked against 0.5.0-rc.8 (tag v0.5.0-rc.8, commit af55a77) on 2026-10-04
- date: 2026-10-04
- spec: ../specs/window.md

## Context

ADR 0005 gave the window no native border and took the resize bands from the toolkit's `BorderlessPlugin`. On a real machine the
window could not be resized and the pointer showed no resize cursor near the edge. Running the app on a Wayland compositor (sway,
headless) at scale 1.0 everything worked; at 1.5 the left and top edges answered and the right, bottom and bottom-right did not.

The cause is in the toolkit (rc.8): `Platform::root_size` is set from `window.inner_size()`, physical pixels, and the plugin's bands
compute the far edges as `root_size - thickness` and place them with `Position::new_global()`, which is in layout units (physical
pixels divided by the display scale and the UI scale). At scale 1.0 the two agree; at 1.5 the right band sat at 1344 on a window 900
wide. The earlier X11 checks ran at scale 1.0, which is why they passed.

## Decision

The plugin keeps rounding the corners, with no thickness so its own bands have no area. `window::ResizeBands`, mounted by the `Shell`
for the custom frame, places eight bands from the size the `Shell` itself measures (layout units, the number the layout classes use).
Their geometry is pure (`window::resize`) and tested with plain numbers. A press on a band is a `WindowCommand::BeginResize(Edge)`, so
components still never call winit, and the band swallows the press: the corner band lies over the title bar's Close button. The bands
are 8 layout units thick and the corners 16, a little more than the toolkit's 6 and 12, because there is no border to aim at.

## Rejected

* **Patching the toolkit** (a fork behind `[patch]`). One line upstream, but the hygiene test pins Freya to the published release so
  that the docs' claims about it stay true, and a fork is a rebase on every release.
* **Keeping the plugin's bands and compensating** (thickness scaled by the display scale). The far bands' positions come from the wrong
  number whatever the thickness is.
* **Native decorations on Wayland.** GNOME gives no border to a client that asks for none, which is why ADR 0005 draws one; the native
  frame stays the escape hatch (`RECLAW_WINDOW_FRAME=native`).
* **Resize handles drawn as a visible grip.** Would work at any scale but puts a mark on every window for a problem with the hit area.

## Consequences

The toolkit's bug should be reported upstream (divide by the scale factor, or use the layout area); when a release fixes it, these
bands can go and the plugin's can come back. `window::bands` and the plugin must not both have a thickness. The bands were checked on
sway at scales 1.0, 1.5 and 2.0 (cursor shape per edge and corner, and real drags); GNOME's compositor was not available, so whether
Mutter behaves the same is unverified.
