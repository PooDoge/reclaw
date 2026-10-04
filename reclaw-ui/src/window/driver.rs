use freya::prelude::*;
use reclaw_games::settings::DisplayServer;
use reclaw_input::UiMode;

use super::{
    platform,
    policy::{DeckDisplay, on_mode, ui_scale},
};
use crate::{
    shell::ShellModel,
    store::{Store, use_display, use_settings},
};

/// Keeps the window in step with the app. A hook for the `Shell` to run on every render:
///
/// * reads the monitors at startup, whenever the window regains focus, and when the scale changes;
/// * remembers the window's size, position and maximized state (the store saves them, debounced);
/// * fills a monitor in Deck mode, and gives the window back on leaving it (`policy::on_mode`);
/// * applies the UI scale row on top of the system's scale factor.
///
/// With nothing `attached` (the headless tests) it asks nothing of any window.
pub fn use_window_driver(store: Store, model: State<ShellModel>, server: DisplayServer, attached: bool) {
    let platform_state = Platform::get();
    let (settings, display) = (use_settings(), use_display());

    // Monitors: once, then when the user comes back to the window, then when the scale changes.
    use_hook(move || {
        if attached {
            platform::refresh_monitors(store, server);
        }
    });
    use_side_effect(move || {
        if attached && *platform_state.is_app_focused.read() {
            platform::refresh_monitors(store, server);
        }
    });

    // Geometry: a resize, a scale change or a focus change is a moment to look at the window. A move
    // alone raises no event of its own, so a new position is saved at the next of these.
    use_side_effect(move || {
        let _ = platform_state.root_size.read();
        let _ = platform_state.scale_factor.read();
        let _ = platform_state.is_app_focused.read();
        if attached {
            platform::record(store);
        }
    });

    // Deck mode fills the screen. `fullscreened` is whether this made the window fullscreen, so that
    // only that is undone.
    let mut fullscreened = use_state(|| false);
    let monitors = display.monitors.len();
    let deck = DeckDisplay::from_settings(&settings);
    use_side_effect(move || {
        let mode: UiMode = model.read().mode;
        let ours = *fullscreened.peek();
        if let Some(command) = on_mode(mode, deck, ours, monitors).filter(|_| attached) {
            fullscreened.set(matches!(command, super::WindowCommand::Fullscreen { .. }));
            platform::run(&command, store);
        }
    });

    // The UI scale row multiplies the system's scale factor.
    let wanted = ui_scale(&settings);
    let mut applied = use_state(|| 1.0_f64);
    if attached && *applied.peek() != wanted {
        applied.set(wanted);
        platform_state.set_custom_scale_factor(wanted);
    }
}

/// Carry out a command the UI emitted. The `Shell` calls this for `Effect::Window`.
pub fn run_command(command: &super::WindowCommand, store: Store) {
    platform::run(command, store);
}
