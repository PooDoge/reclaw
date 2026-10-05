//! The winit side: read the monitors and the window, and carry out a [`WindowCommand`]. Nothing here
//! decides anything; the rules are in `monitors`, `geometry` and `policy`.
use freya::{
    prelude::*,
    winit::{
        event_loop::ActiveEventLoop,
        monitor::MonitorHandle,
        window::{Fullscreen, ResizeDirection, Window},
    },
};
use futures_channel::oneshot;

use super::{
    command::WindowCommand,
    geometry::{Snapshot, remember},
    monitors::{RawMonitor, arrange, id_at, order_key},
    resize::Edge,
};
use crate::store::{AppAction, Store};

fn raw(handle: &MonitorHandle, primary: Option<&MonitorHandle>) -> RawMonitor {
    let (size, position) = (handle.size(), handle.position());
    RawMonitor {
        name: handle.name(),
        size: (size.width, size.height),
        position: (position.x, position.y),
        scale: handle.scale_factor(),
        refresh_mhz: handle.refresh_rate_millihertz(),
        primary: primary == Some(handle),
    }
}

/// Every monitor with its handle, in the order monitors are numbered.
fn arranged(handles: impl Iterator<Item = MonitorHandle>, primary: Option<MonitorHandle>) -> Vec<(MonitorHandle, RawMonitor)> {
    let mut all: Vec<_> = handles
        .map(|h| {
            let r = raw(&h, primary.as_ref());
            (h, r)
        })
        .collect();
    all.sort_by_key(|(_, r)| order_key(r));
    all
}

/// The monitors, before the window exists: the launch hook has the event loop and nothing else.
pub(super) fn monitors_of_event_loop(event_loop: &ActiveEventLoop) -> Vec<RawMonitor> {
    arranged(event_loop.available_monitors(), event_loop.primary_monitor()).into_iter().map(|(_, r)| r).collect()
}

fn monitors_of_window(window: &Window) -> Vec<RawMonitor> {
    arranged(window.available_monitors(), window.primary_monitor()).into_iter().map(|(_, r)| r).collect()
}

/// The window as it is now, with the monitor it is on named the way the settings name monitors.
fn snapshot(window: &Window) -> Snapshot {
    let scale = window.scale_factor();
    let inner = window.inner_size();
    let monitors = arrange(monitors_of_window(window));
    let monitor = window.current_monitor().and_then(|h| {
        let (p, s) = (h.position(), h.size());
        id_at(&monitors, (p.x, p.y), (s.width, s.height))
    });
    Snapshot {
        size: ((f64::from(inner.width) / scale) as f32, (f64::from(inner.height) / scale) as f32),
        // Wayland cannot say where a window is; the error is the answer, not a failure.
        position: window.outer_position().ok().map(|p| (p.x, p.y)),
        maximized: window.is_maximized(),
        fullscreen: window.fullscreen().is_some(),
        monitor,
    }
}

/// Ask the window something and get the answer back on the UI side. The winit window lives on the
/// event loop, so a question is posted to it; in the headless test runner it is never answered.
fn ask<T: 'static>(read: impl FnOnce(&Window) -> T + 'static) -> oneshot::Receiver<T> {
    let (tx, rx) = oneshot::channel();
    Platform::get().with_window(Platform::window_id(), move |window| {
        // The receiver is gone when the asker no longer cares; nothing to do about it.
        let _ = tx.send(read(window));
    });
    rx
}

/// Read the monitors and tell the store. Run when the window is created, when it gains focus (the
/// moment someone has plugged a monitor in) and when the scale changes.
pub(super) fn refresh_monitors(store: Store, server: reclaw_games::settings::DisplayServer) {
    let answer = ask(monitors_of_window);
    spawn(async move {
        if let Ok(monitors) = answer.await {
            store.dispatch(AppAction::SetDisplay(super::monitors::environment(server, &monitors)));
        }
    });
}

/// Whether the window fills the screen (maximized or fullscreen), kept current as it is resized.
/// The toolkit's own `use_maximized` asks the window for its id while rendering, which a headless
/// run does not have, so this one asks only when a window is `attached`.
pub(super) fn use_maximized(attached: bool) -> State<bool> {
    let mut maximized = use_state(|| false);
    let platform = Platform::get();
    use_side_effect(move || {
        // A maximize or restore changes the size, which is when to look again.
        let _ = platform.root_size.read();
        if attached {
            platform.with_window(Platform::window_id(), move |window| {
                if let Some(mut flag) = maximized.try_write() {
                    *flag = window.fullscreen().is_some() || window.is_maximized();
                }
            });
        }
    });
    maximized
}

/// Write the window's state into the preferences if it is worth saving. Returns once the store has it.
fn save(store: Store, snapshot: &Snapshot) {
    let current = store.with(|s| s.window.clone());
    if let Some(next) = remember(&current, snapshot) {
        store.dispatch(AppAction::Window(next));
    }
}

/// Look at the window and remember what changed.
pub(super) fn record(store: Store) {
    let answer = ask(snapshot);
    spawn(async move {
        if let Ok(snapshot) = answer.await {
            save(store, &snapshot);
        }
    });
}

fn direction_of(edge: Edge) -> ResizeDirection {
    match edge {
        Edge::North => ResizeDirection::North,
        Edge::South => ResizeDirection::South,
        Edge::West => ResizeDirection::West,
        Edge::East => ResizeDirection::East,
        Edge::NorthWest => ResizeDirection::NorthWest,
        Edge::NorthEast => ResizeDirection::NorthEast,
        Edge::SouthWest => ResizeDirection::SouthWest,
        Edge::SouthEast => ResizeDirection::SouthEast,
    }
}

/// Carry out a command. `Close` records the window first and writes the preferences before it
/// closes: the toolkit's own close call does not run the host's close hook, which is where the
/// preferences are normally written.
pub(super) fn run(command: &WindowCommand, store: Store) {
    let platform = Platform::get();
    let id = Platform::window_id();
    match command {
        WindowCommand::Minimize => platform.with_window(id, |window| window.set_minimized(true)),
        WindowCommand::ToggleMaximize => platform.with_window(id, |window| window.set_maximized(!window.is_maximized())),
        WindowCommand::Fullscreen { monitor } => {
            let wanted = *monitor;
            platform.with_window(id, move |window| {
                // With no handle winit fills the monitor the window is on.
                let target =
                    wanted.and_then(|i| arranged(window.available_monitors(), window.primary_monitor()).into_iter().nth(i)).map(|(h, _)| h);
                window.set_fullscreen(Some(Fullscreen::Borderless(target)));
            });
        }
        WindowCommand::Windowed => platform.with_window(id, |window| window.set_fullscreen(None)),
        WindowCommand::BeginResize(edge) => {
            let direction = direction_of(*edge);
            platform.with_window(id, move |window| {
                if let Err(e) = window.drag_resize_window(direction) {
                    tracing::warn!(error = %e, "the window system would not start a resize");
                }
            });
        }
        WindowCommand::Close => {
            let answer = ask(snapshot);
            spawn(async move {
                if let Ok(snapshot) = answer.await {
                    save(store, &snapshot);
                }
                store.flush();
                platform.close_window(id);
            });
        }
    }
}
