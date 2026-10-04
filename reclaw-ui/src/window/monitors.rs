//! Monitors as the windowing system reports them, turned into what the rest of the app uses: the
//! `DisplayEnvironment` the launch settings read, and the "Monitor 2" numbering people use.
use reclaw_games::settings::{DisplayEnvironment, DisplayServer, Monitor, Size};

/// One monitor's facts. `platform` fills these from winit; tests write them by hand.
#[derive(Clone, PartialEq, Debug)]
pub struct RawMonitor {
    /// The connector or output name when the system gives one ("DP-1").
    pub name: Option<String>,
    /// The current mode in physical pixels.
    pub size: (u32, u32),
    /// Top-left on the virtual desktop, in physical pixels.
    pub position: (i32, i32),
    pub scale: f64,
    pub refresh_mhz: Option<u32>,
    /// The system says this is the primary. Wayland never does.
    pub primary: bool,
}

impl RawMonitor {
    /// The size in logical pixels, which is what a window's size is measured in.
    pub fn logical_size(&self) -> (f32, f32) {
        let scale = if self.scale > 0. { self.scale as f32 } else { 1. };
        (self.size.0 as f32 / scale, self.size.1 as f32 / scale)
    }
}

/// What monitors are ordered by: left to right, then top to bottom. `platform` sorts the matching
/// winit handles by the same key, so a number means the same monitor on both sides.
pub fn order_key(m: &RawMonitor) -> (i32, i32) {
    m.position
}

/// The order "Monitor 1, 2, 3" follows. Stable, so monitors at the same spot (a mirrored pair) keep
/// the order the system listed them in.
pub fn arrange(mut monitors: Vec<RawMonitor>) -> Vec<RawMonitor> {
    monitors.sort_by_key(order_key);
    monitors
}

/// The label for a monitor's place in the order: 0 is "Monitor 1".
pub fn monitor_number(index: usize) -> String {
    format!("Monitor {}", index + 1)
}

/// Which windowing system this session runs, from the environment variables the session sets. The
/// order matters: gamescope sets Wayland variables too.
pub fn detect_server(get: impl Fn(&str) -> Option<String>) -> DisplayServer {
    if cfg!(target_os = "windows") {
        return DisplayServer::Windows;
    }
    if cfg!(target_os = "macos") {
        return DisplayServer::MacOs;
    }
    let present = |k: &str| get(k).is_some_and(|v| !v.is_empty());
    let session = get("XDG_SESSION_TYPE").map(|v| v.to_lowercase());
    let desktop = get("XDG_CURRENT_DESKTOP").map(|v| v.to_lowercase());
    if present("GAMESCOPE_WAYLAND_DISPLAY") || desktop.as_deref().is_some_and(|d| d.contains("gamescope")) {
        DisplayServer::Gamescope
    } else if session.as_deref() == Some("wayland") || present("WAYLAND_DISPLAY") {
        DisplayServer::Wayland
    } else if session.as_deref() == Some("x11") || present("DISPLAY") {
        DisplayServer::X11
    } else {
        DisplayServer::Unknown
    }
}

/// The ids the launch settings save for monitors: the connector name, made unique, or the monitor's
/// number when the system gave no name.
fn ids(monitors: &[RawMonitor]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for (i, m) in monitors.iter().enumerate() {
        let base = m.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).map_or_else(|| format!("monitor-{}", i + 1), str::to_string);
        let mut id = base.clone();
        let mut n = 2;
        while seen.contains(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        seen.push(id);
    }
    seen
}

/// Which monitor is the primary. The system's own answer when it gives one; otherwise the one at the
/// desktop's origin, which is where both X11 and Windows put it by default; otherwise the first.
/// This is a guess on Wayland.
fn primary_index(monitors: &[RawMonitor]) -> usize {
    monitors.iter().position(|m| m.primary).or_else(|| monitors.iter().position(|m| m.position == (0, 0))).unwrap_or(0)
}

/// The environment the launch settings read, from the monitors in any order.
pub fn environment(server: DisplayServer, monitors: &[RawMonitor]) -> DisplayEnvironment {
    let monitors = arrange(monitors.to_vec());
    let ids = ids(&monitors);
    let primary = primary_index(&monitors);
    DisplayEnvironment {
        server,
        monitors: monitors
            .iter()
            .zip(ids)
            .enumerate()
            .map(|(i, (m, id))| Monitor {
                name: m.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| monitor_number(i)),
                id,
                native: Size::new(m.size.0, m.size.1),
                refresh_mhz: m.refresh_mhz.unwrap_or(0),
                primary: i == primary,
            })
            .collect(),
    }
}

/// The id of the monitor under a window, found by where it is. `monitors` must be in `arrange` order.
pub fn id_at(monitors: &[RawMonitor], position: (i32, i32), size: (u32, u32)) -> Option<String> {
    let i = monitors.iter().position(|m| m.position == position && m.size == size)?;
    ids(monitors).into_iter().nth(i)
}

#[cfg(test)]
mod tests;
