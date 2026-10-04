//! Opening the window where it was, without ever opening it where it cannot be reached, and deciding
//! when a change in the window is worth saving.
use reclaw_config::WindowPrefs;
use reclaw_games::settings::DisplayServer;

use super::monitors::RawMonitor;

/// The smallest the window can be made. Below this the layouts have nothing sensible to show.
pub const MIN_SIZE: (f32, f32) = (640., 400.);
/// The size on a first run, and when the saved one is unusable.
pub const DEFAULT_SIZE: (f32, f32) = (1280., 800.);
/// A bigger window than this is a corrupt file, not a wish.
const MAX_SIZE: f32 = 16_384.;
/// How much of the window's top edge must be on some monitor for it to be grabbable.
const GRAB_WIDTH: i64 = 100;
const GRAB_HEIGHT: i64 = 32;

/// A saved size made safe to open with: a missing, non-finite or absurd value becomes the default,
/// and the rest is kept within the limits and whole pixels (a fractional size would save a new
/// value on every resize event).
pub fn sane_size(saved: Option<(f32, f32)>) -> (f32, f32) {
    match saved {
        Some((w, h)) if w.is_finite() && h.is_finite() && w > 0. && h > 0. => {
            (w.round().clamp(MIN_SIZE.0, MAX_SIZE), h.round().clamp(MIN_SIZE.1, MAX_SIZE))
        }
        _ => DEFAULT_SIZE,
    }
}

/// How to open the window, once the monitors are known.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Restore {
    /// A smaller size than the saved one, when the saved one is larger than any monitor.
    pub size: Option<(f32, f32)>,
    /// Where to put the window's top-left, in physical pixels. Only where the system lets a program
    /// place its windows, and only where that lands on a connected monitor.
    pub position: Option<(i32, i32)>,
    pub maximized: bool,
}

/// Whether a window's top edge, from `position` and `width` (physical pixels), can be reached on some monitor.
fn grabbable(position: (i32, i32), width: u32, monitors: &[RawMonitor]) -> bool {
    let (left, top) = (i64::from(position.0), i64::from(position.1));
    let right = left + i64::from(width);
    monitors.iter().any(|m| {
        let (m_left, m_top) = (i64::from(m.position.0), i64::from(m.position.1));
        let (m_right, m_bottom) = (m_left + i64::from(m.size.0), m_top + i64::from(m.size.1));
        let overlap_x = right.min(m_right) - left.max(m_left);
        let overlap_y = (top + GRAB_HEIGHT).min(m_bottom) - top.max(m_top);
        overlap_x >= GRAB_WIDTH.min(i64::from(width)) && overlap_y >= GRAB_HEIGHT
    })
}

/// What to do with the saved window now that the monitors are known.
///
/// Wayland compositors choose where windows go, so a saved position is only used where a program may
/// place its own windows (X11, Windows). On Wayland the window keeps its size and the compositor
/// decides the rest.
pub fn restore(prefs: &WindowPrefs, server: DisplayServer, monitors: &[RawMonitor]) -> Restore {
    let size = sane_size(prefs.size);
    let largest = monitors.iter().map(RawMonitor::logical_size).fold((0f32, 0f32), |a, s| (a.0.max(s.0), a.1.max(s.1)));
    // A window wider or taller than every monitor is brought down to the biggest one.
    let fitted = (largest.0 > 0. && largest.1 > 0. && (size.0 > largest.0 || size.1 > largest.1))
        .then(|| (size.0.min(largest.0).max(MIN_SIZE.0), size.1.min(largest.1).max(MIN_SIZE.1)));
    let may_place = matches!(server, DisplayServer::X11 | DisplayServer::Windows);
    let position = prefs.position.filter(|_| may_place).filter(|p| {
        // The scale of the monitor it was on is not saved, so the check uses the width in logical pixels
        // as physical ones: generous, and what matters is that some part of the bar is reachable.
        grabbable(*p, size.0 as u32, monitors)
    });
    Restore { size: fitted, position, maximized: prefs.maximized }
}

/// What the window looks like now.
#[derive(Clone, PartialEq, Debug)]
pub struct Snapshot {
    /// Logical size.
    pub size: (f32, f32),
    pub position: Option<(i32, i32)>,
    pub maximized: bool,
    pub fullscreen: bool,
    /// The id of the monitor it is on.
    pub monitor: Option<String>,
}

/// The prefs after seeing `now`, or `None` when nothing worth saving changed.
///
/// While the window is maximized or fullscreen its size and position are the screen's, not the
/// user's choice, so only the flag and the monitor are kept and the restored size survives. Fullscreen
/// (Deck mode) changes nothing at all: coming back to the desktop should find the window as it was.
pub fn remember(prefs: &WindowPrefs, now: &Snapshot) -> Option<WindowPrefs> {
    if now.fullscreen {
        return None;
    }
    let mut next = prefs.clone();
    next.maximized = now.maximized;
    if now.monitor.is_some() {
        next.monitor.clone_from(&now.monitor);
    }
    if !now.maximized {
        next.size = Some((now.size.0.round(), now.size.1.round()));
        if now.position.is_some() {
            next.position = now.position;
        }
    }
    (next != *prefs).then_some(next)
}

#[cfg(test)]
mod tests;
