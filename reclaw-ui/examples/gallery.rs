//! `cargo run --example gallery` opens the Library page. Resize the window across 720px and
//! 1100px to see the three layout classes.
//!
//! RECLAW_THEME=daylight and RECLAW_DENSITY=touch switch theme and density.
use freya::prelude::*;
use reclaw_ui::{
    app::ReclawApp,
    metrics::Density,
    sample::{sample_downloads, sample_games},
    theme::ThemeKind,
};

fn main() {
    let theme = match std::env::var("RECLAW_THEME").as_deref() {
        Ok("daylight") => ThemeKind::Daylight,
        _ => ThemeKind::Midnight,
    };
    let density = match std::env::var("RECLAW_DENSITY").as_deref() {
        Ok("touch") => Some(Density::Touch),
        Ok("pointer") => Some(Density::Pointer),
        _ => None,
    };
    let app = ReclawApp {
        games: sample_games(),
        downloads: sample_downloads(),
        theme,
        density,
    };
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new_app(app)
                .with_title("Reclaw")
                .with_size(1100., 700.)
                .with_min_size(360., 560.),
        ),
    );
}
