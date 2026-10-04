The two slide-in panels. Rust: `reclaw_ui::deck::{SlidePanel, MainMenu, QuickAccess}`.

They overlay the page and never shift it: an absolute root on `Layer::Overlay`, the `scrim` token over the rest of the window, 280ms expo-out slide. They trap focus; Back closes. Opening one over a running app (Guide) brings the launcher forward and takes the pad; closing it returns the pad to the app. Main menu rows are `deck-row-h` tall with an accent bar on the focused row (no scale, so rows stay aligned).
