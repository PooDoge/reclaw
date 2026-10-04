//! Switches for developers and testers, read from the environment so any build can be pushed into
//! any form factor without a device: `RECLAW_MODE`, `RECLAW_LAYOUT`, `RECLAW_DENSITY`,
//! `RECLAW_KEYBOARD`, `RECLAW_SIM_KEYBOARD`, `RECLAW_MOTION` and `RECLAW_THEME`. (`RECLAW_MODE` itself is read by
//! `reclaw_input::detect_environment`.)
use crate::{
    metrics::{Density, LayoutClass},
    nav::transition::{Intensity, TransitionConfig},
    theme::ThemeKind,
};

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct DevOverrides {
    /// Force a layout class on the desktop interface, whatever the window width.
    pub layout: Option<LayoutClass>,
    pub density: Option<Density>,
    /// Start with the simulated keyboard up. `auto` means a default share of the window height.
    pub keyboard: Option<KeyboardStart>,
    /// Raise the simulated keyboard whenever a text field takes focus.
    pub keyboard_follows_focus: bool,
    pub theme: Option<ThemeKind>,
    pub motion: Option<MotionOverride>,
}

/// `RECLAW_MOTION`: `reduced` (or `off`) cuts every transition; `subtle`, `standard` or `cinematic`
/// sets both interfaces to that intensity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MotionOverride {
    Reduced,
    Intensity(Intensity),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyboardStart {
    Share,
    Pixels(f32),
}

impl DevOverrides {
    /// `get` is injected so this is testable. Unrecognised values are ignored, not errors: these
    /// are conveniences, and a typo must not stop the app starting.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Self {
        let value = |k: &str| get(k).map(|v| v.trim().to_lowercase());
        Self {
            layout: value("RECLAW_LAYOUT").and_then(|v| match v.as_str() {
                "wide" => Some(LayoutClass::Wide),
                "compact" => Some(LayoutClass::Compact),
                "phone" => Some(LayoutClass::Phone),
                _ => None,
            }),
            density: value("RECLAW_DENSITY").and_then(|v| match v.as_str() {
                "pointer" => Some(Density::Pointer),
                "touch" => Some(Density::Touch),
                "controller" => Some(Density::Controller),
                _ => None,
            }),
            keyboard: value("RECLAW_KEYBOARD").and_then(|v| match v.as_str() {
                "auto" | "on" => Some(KeyboardStart::Share),
                px => px.parse::<f32>().ok().filter(|px| *px > 0.).map(KeyboardStart::Pixels),
            }),
            keyboard_follows_focus: value("RECLAW_SIM_KEYBOARD").is_some_and(|v| !matches!(v.as_str(), "" | "0" | "off" | "false")),
            theme: value("RECLAW_THEME").and_then(|v| match v.as_str() {
                "midnight" => Some(ThemeKind::Midnight),
                "daylight" => Some(ThemeKind::Daylight),
                _ => None,
            }),
            motion: value("RECLAW_MOTION").and_then(|v| match v.as_str() {
                "reduced" | "off" | "none" => Some(MotionOverride::Reduced),
                "subtle" => Some(MotionOverride::Intensity(Intensity::Subtle)),
                "standard" => Some(MotionOverride::Intensity(Intensity::Standard)),
                "cinematic" => Some(MotionOverride::Intensity(Intensity::Cinematic)),
                _ => None,
            }),
        }
    }

    /// The starting transition settings: the defaults, changed by `RECLAW_MOTION`.
    pub fn transitions(&self) -> TransitionConfig {
        let mut config = TransitionConfig::default();
        match self.motion {
            Some(MotionOverride::Reduced) => config.reduce_motion = true,
            Some(MotionOverride::Intensity(i)) => {
                config.desktop.intensity = i;
                config.console.intensity = i;
            }
            None => {}
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from(pairs: &'static [(&str, &str)]) -> DevOverrides {
        DevOverrides::from_env(|k| pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string()))
    }

    #[test]
    fn nothing_set_changes_nothing() {
        assert_eq!(from(&[]), DevOverrides::default());
    }

    #[test]
    fn each_variable_is_read_case_insensitively() {
        let d = from(&[
            ("RECLAW_LAYOUT", "Phone"),
            ("RECLAW_DENSITY", "TOUCH"),
            ("RECLAW_KEYBOARD", "320"),
            ("RECLAW_SIM_KEYBOARD", "1"),
            ("RECLAW_THEME", "daylight"),
        ]);
        assert_eq!(d.layout, Some(LayoutClass::Phone));
        assert_eq!(d.density, Some(Density::Touch));
        assert_eq!(d.keyboard, Some(KeyboardStart::Pixels(320.)));
        assert!(d.keyboard_follows_focus);
        assert_eq!(d.theme, Some(ThemeKind::Daylight));
    }

    #[test]
    fn motion_overrides_change_the_starting_transitions() {
        assert_eq!(from(&[]).transitions(), TransitionConfig::default());
        assert!(from(&[("RECLAW_MOTION", "reduced")]).transitions().reduce_motion);
        let cinematic = from(&[("RECLAW_MOTION", "Cinematic")]).transitions();
        assert_eq!((cinematic.desktop.intensity, cinematic.console.intensity), (Intensity::Cinematic, Intensity::Cinematic));
    }

    #[test]
    fn keyboard_auto_means_a_share_of_the_window() {
        assert_eq!(from(&[("RECLAW_KEYBOARD", "auto")]).keyboard, Some(KeyboardStart::Share));
    }

    #[test]
    fn typos_and_nonsense_are_ignored() {
        let d = from(&[("RECLAW_LAYOUT", "huge"), ("RECLAW_KEYBOARD", "-5"), ("RECLAW_SIM_KEYBOARD", "0"), ("RECLAW_DENSITY", "")]);
        assert_eq!(d, DevOverrides::default());
    }
}
