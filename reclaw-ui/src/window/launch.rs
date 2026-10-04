//! What a host launches with: the window configuration and plugin that make the custom frame work,
//! and the first look at the monitors, which only the window-creation hook can give (the window does
//! not exist yet, so the event loop is asked).
use freya::{
    borderless::BorderlessPlugin,
    prelude::*,
    winit::dpi::{LogicalSize, PhysicalPosition},
};
use reclaw_config::WindowPrefs;
use reclaw_games::settings::{DisplayEnvironment, DisplayServer};

use super::{
    frame::Frame,
    geometry::{MIN_SIZE, restore, sane_size},
    monitors::environment,
    platform::monitors_of_event_loop,
    policy::ui_scale,
};
use crate::store::{AppAction, Store};

/// Matches the `.desktop` file, so GNOME and KDE group the window under the right icon and name.
pub const APP_ID: &str = "dev.reclaw.Reclaw";
/// How round the custom frame's corners are, in logical pixels.
const CORNER_RADIUS: f32 = 12.;

/// Everything the window needs to be created the way it was left.
pub struct Setup {
    pub frame: Frame,
    pub prefs: WindowPrefs,
    pub server: DisplayServer,
    /// The UI scale row, on top of the system's scale factor.
    pub scale: f64,
}

/// The launch configuration for a frame: the borderless plugin (resize bands, rounded corners) for
/// the custom frame, nothing for the native one.
pub fn with_frame(config: LaunchConfig, frame: Frame) -> LaunchConfig {
    match frame {
        Frame::Custom => config.with_plugin(BorderlessPlugin::new().with_corner_radius(CORNER_RADIUS)),
        Frame::Native => config,
    }
}

/// Apply the frame, the saved size, the saved place and the scale to a window configuration. The
/// monitors read at creation go to `on_display`, so the first frame already knows the screens.
pub fn style(config: WindowConfig, setup: Setup, on_display: impl FnOnce(DisplayEnvironment) + 'static) -> WindowConfig {
    let (width, height) = sane_size(setup.prefs.size);
    let custom = setup.frame == Frame::Custom;
    let Setup { prefs, server, scale, .. } = setup;
    config
        .with_size(f64::from(width), f64::from(height))
        .with_min_size(f64::from(MIN_SIZE.0), f64::from(MIN_SIZE.1))
        .with_decorations(!custom)
        .with_transparency(custom)
        .with_background(if custom { Color::TRANSPARENT } else { Color::BLACK })
        .with_app_id(APP_ID)
        .with_custom_scale_factor(scale)
        .with_window_attributes(move |attributes, event_loop| {
            let monitors = monitors_of_event_loop(event_loop);
            let place = restore(&prefs, server, &monitors);
            on_display(environment(server, &monitors));
            let mut attributes = attributes.with_maximized(place.maximized);
            if let Some((w, h)) = place.size {
                attributes = attributes.with_inner_size(LogicalSize::new(f64::from(w), f64::from(h)));
            }
            if let Some((x, y)) = place.position {
                attributes = attributes.with_position(PhysicalPosition::new(x, y));
            }
            attributes
        })
}

/// The whole launch for an app: its window styled for `frame` and opened where it was left, the
/// borderless plugin, the monitors handed to the store before the first frame, and the preferences
/// flushed when the window closes. The one call a host needs.
pub fn launch_config(app: impl App + 'static, store: Store, frame: Frame, server: DisplayServer) -> LaunchConfig {
    let (prefs, scale) = store.with(|s| (s.window.clone(), ui_scale(&s.settings)));
    let window = style(WindowConfig::new_app(app).with_title("Reclaw"), Setup { frame, prefs, server, scale }, move |displays| {
        store.dispatch(AppAction::SetDisplay(displays));
    })
    .with_on_close(move |_, _| {
        store.flush();
        CloseDecision::Close
    });
    with_frame(LaunchConfig::new().with_window(window), frame)
}
