use super::{
    environment::{DisplayEnvironment, DisplayServer},
    key::SettingKey,
    value::{Choice, Size, ValueKind},
};

/// The resolutions the launcher lists, smallest first. The monitor's native size trims the list.
const STANDARD_SIZES: [Size; 8] = [
    Size::new(1280, 720),
    Size::new(1366, 768),
    Size::new(1600, 900),
    Size::new(1920, 1080),
    Size::new(2560, 1080),
    Size::new(2560, 1440),
    Size::new(3440, 1440),
    Size::new(3840, 2160),
];

fn choices(items: &[(&str, &str)]) -> ValueKind {
    ValueKind::Choice(items.iter().map(|(id, label)| Choice::new(id, label)).collect())
}

fn window_modes(server: DisplayServer) -> Option<ValueKind> {
    match server {
        // The compositor owns the screen; the game cannot be asked to take it exclusively.
        DisplayServer::Gamescope => None,
        DisplayServer::Wayland => Some(choices(&[("windowed", "Windowed"), ("borderless", "Borderless")])),
        _ => Some(choices(&[("windowed", "Windowed"), ("borderless", "Borderless"), ("exclusive", "Exclusive fullscreen")])),
    }
}

fn resolutions(env: &DisplayEnvironment) -> ValueKind {
    let options = match env.primary() {
        Some(monitor) => {
            STANDARD_SIZES.into_iter().filter(|s| s.width <= monitor.native.width && s.height <= monitor.native.height).collect()
        }
        None => STANDARD_SIZES.to_vec(),
    };
    ValueKind::Size { options, native: true }
}

/// Only worth asking with a choice of screens, and never when one compositor owns them all.
fn monitors(env: &DisplayEnvironment) -> Option<ValueKind> {
    if env.server == DisplayServer::Gamescope || env.monitors.len() < 2 {
        return None;
    }
    Some(ValueKind::Choice(
        env.monitors.iter().map(|m| Choice { id: m.id.clone(), label: format!("{} ({})", m.name, m.native.label()) }).collect(),
    ))
}

/// The shape of `key`'s values on this display, before any game narrows it. `None` when the display
/// cannot honor the setting at all (a monitor choice under gamescope, exclusive fullscreen is
/// removed from the window modes on Wayland instead).
pub fn standard_kind(key: SettingKey, env: &DisplayEnvironment) -> Option<ValueKind> {
    Some(match key {
        SettingKey::WindowMode => return window_modes(env.server),
        SettingKey::Resolution => resolutions(env),
        SettingKey::AspectRatio => {
            choices(&[("auto", "Auto"), ("4:3", "4:3"), ("16:9", "16:9"), ("16:10", "16:10"), ("21:9", "21:9"), ("32:9", "32:9")])
        }
        SettingKey::Monitor => return monitors(env),
        SettingKey::Vsync => ValueKind::Bool,
        SettingKey::FrameLimit => ValueKind::Int { min: 0, max: 360, step: 1, unit: Some("fps"), zero_label: Some("No limit") },
        SettingKey::UpscaleMethod => choices(&[("off", "Off"), ("linear", "Linear"), ("integer", "Integer scaling"), ("fsr1", "FSR 1.0")]),
        SettingKey::RenderScale => ValueKind::Int { min: 25, max: 200, step: 5, unit: Some("%"), zero_label: None },
        SettingKey::Sharpness => ValueKind::Int { min: 0, max: 100, step: 5, unit: Some("%"), zero_label: None },
        SettingKey::Msaa => choices(&[("off", "Off"), ("2x", "2x"), ("4x", "4x"), ("8x", "8x")]),
        SettingKey::TextureFilter => choices(&[
            ("nearest", "Nearest"),
            ("linear", "Linear"),
            ("trilinear", "Trilinear"),
            ("aniso4", "Anisotropic 4x"),
            ("aniso16", "Anisotropic 16x"),
        ]),
    })
}

#[cfg(test)]
mod tests;
