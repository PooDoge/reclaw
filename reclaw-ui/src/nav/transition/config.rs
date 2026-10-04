use std::time::Duration;

use reclaw_input::UiMode;

use super::style::{Direction, Easing, Intensity, TransitionStyle};
use crate::nav::Route;

/// One interface's transition settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModeProfile {
    pub intensity: Intensity,
    /// Used when the page being shown has no preference of its own.
    pub style: TransitionStyle,
    /// Let pages pick their own style (a game page's hero entrance). Off means every page uses `style`.
    pub page_styles: bool,
}

/// The transition settings of the whole app: one profile per interface, plus the reduced-motion
/// switch that overrides both.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TransitionConfig {
    pub desktop: ModeProfile,
    pub console: ModeProfile,
    pub reduce_motion: bool,
}

impl Default for TransitionConfig {
    /// Calm on a desktop where people click quickly, livelier on a TV where the view is the show.
    fn default() -> Self {
        Self {
            desktop: ModeProfile { intensity: Intensity::Subtle, style: TransitionStyle::Slide, page_styles: true },
            console: ModeProfile { intensity: Intensity::Standard, style: TransitionStyle::Slide, page_styles: true },
            reduce_motion: false,
        }
    }
}

impl TransitionConfig {
    pub fn profile(&self, mode: UiMode) -> &ModeProfile {
        match mode {
            UiMode::Desktop => &self.desktop,
            UiMode::Deck => &self.console,
        }
    }
}

/// A fully decided transition: what `RouteStage` plays and what a page can read.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Spec {
    pub style: TransitionStyle,
    pub direction: Direction,
    pub duration: Duration,
    pub distance: f32,
    pub easing: Easing,
}

impl Spec {
    pub fn none(direction: Direction) -> Self {
        Self { style: TransitionStyle::None, direction, duration: Duration::ZERO, distance: 0., easing: Easing::ExpoOut }
    }

    pub fn is_none(&self) -> bool {
        self.style == TransitionStyle::None || self.duration.is_zero()
    }
}

fn duration_scale(style: TransitionStyle) -> f32 {
    match style {
        TransitionStyle::Hero => 1.5,
        TransitionStyle::Fade => 0.8,
        TransitionStyle::Zoom => 0.9,
        TransitionStyle::None | TransitionStyle::Slide | TransitionStyle::Rise => 1.,
    }
}

/// Decide how `from` gives way to `to`.
///
/// Order of precedence, strongest first: reduced motion or an Off intensity cuts; switching tabs
/// cross-fades; going forward the page being opened picks its own style, going back the page being
/// left does (played in reverse); otherwise the interface's default style.
pub fn resolve(config: &TransitionConfig, mode: UiMode, from: &Route, to: &Route, direction: Direction) -> Spec {
    let profile = config.profile(mode);
    if config.reduce_motion || profile.intensity == Intensity::Off {
        return Spec::none(direction);
    }
    let style = match direction {
        Direction::Replace => TransitionStyle::Fade,
        Direction::Forward => profile.page_styles.then(|| to.enter_style()).flatten().unwrap_or(profile.style),
        Direction::Back => profile.page_styles.then(|| from.enter_style()).flatten().unwrap_or(profile.style),
    };
    if style == TransitionStyle::None {
        return Spec::none(direction);
    }
    let ms = (profile.intensity.duration_ms() as f32 * duration_scale(style)).round() as u64;
    Spec { style, direction, duration: Duration::from_millis(ms), distance: profile.intensity.distance(), easing: Easing::ExpoOut }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Route {
        Route::Game { id: 1 }
    }

    fn at(config: &TransitionConfig, mode: UiMode, from: &Route, to: &Route, direction: Direction) -> Spec {
        resolve(config, mode, from, to, direction)
    }

    #[test]
    fn off_and_reduced_motion_cut() {
        let mut config = TransitionConfig::default();
        config.console.intensity = Intensity::Off;
        assert!(at(&config, UiMode::Deck, &Route::Library {}, &game(), Direction::Forward).is_none());
        assert!(!at(&config, UiMode::Desktop, &Route::Library {}, &game(), Direction::Forward).is_none(), "desktop is separate");

        let reduced = TransitionConfig { reduce_motion: true, ..TransitionConfig::default() };
        assert!(at(&reduced, UiMode::Desktop, &Route::Library {}, &game(), Direction::Forward).is_none());
    }

    #[test]
    fn a_game_page_asks_for_the_hero_style_forward_and_back() {
        let config = TransitionConfig::default();
        let forward = at(&config, UiMode::Desktop, &Route::Library {}, &game(), Direction::Forward);
        let back = at(&config, UiMode::Desktop, &game(), &Route::Library {}, Direction::Back);
        assert_eq!((forward.style, back.style), (TransitionStyle::Hero, TransitionStyle::Hero));
        assert_eq!(forward.direction, Direction::Forward);
        assert_eq!(back.direction, Direction::Back);
    }

    #[test]
    fn pages_without_a_preference_use_the_interface_default() {
        let mut config = TransitionConfig::default();
        config.console.style = TransitionStyle::Zoom;
        let spec = at(&config, UiMode::Deck, &Route::Library {}, &Route::Settings {}, Direction::Forward);
        assert_eq!(spec.style, TransitionStyle::Zoom);
        let desktop = at(&config, UiMode::Desktop, &Route::Library {}, &Route::Settings {}, Direction::Forward);
        assert_eq!(desktop.style, TransitionStyle::Slide);
    }

    #[test]
    fn turning_page_styles_off_makes_every_page_use_the_default() {
        let mut config = TransitionConfig::default();
        config.desktop.page_styles = false;
        let spec = at(&config, UiMode::Desktop, &Route::Library {}, &game(), Direction::Forward);
        assert_eq!(spec.style, TransitionStyle::Slide);
    }

    #[test]
    fn switching_tabs_cross_fades_faster_than_a_slide() {
        let config = TransitionConfig::default();
        let tab = at(&config, UiMode::Desktop, &Route::Library {}, &Route::Catalog {}, Direction::Replace);
        let slide = at(&config, UiMode::Desktop, &Route::Library {}, &Route::Settings {}, Direction::Forward);
        assert_eq!(tab.style, TransitionStyle::Fade);
        assert!(tab.duration < slide.duration);
    }

    #[test]
    fn intensity_scales_duration_and_distance() {
        let mut config = TransitionConfig::default();
        let calm = at(&config, UiMode::Desktop, &Route::Library {}, &Route::Settings {}, Direction::Forward);
        config.desktop.intensity = Intensity::Cinematic;
        let big = at(&config, UiMode::Desktop, &Route::Library {}, &Route::Settings {}, Direction::Forward);
        assert!(big.duration > calm.duration && big.distance > calm.distance);
    }

    #[test]
    fn a_style_of_none_cuts() {
        let mut config = TransitionConfig::default();
        config.desktop = ModeProfile { style: TransitionStyle::None, page_styles: false, ..config.desktop };
        assert!(at(&config, UiMode::Desktop, &Route::Library {}, &game(), Direction::Forward).is_none());
    }
}
