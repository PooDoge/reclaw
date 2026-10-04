/// Which way a navigation went, which decides which way things move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    /// A new page opened on top: the stack grew.
    Forward,
    /// Back, or the mouse's back button.
    Back,
    /// Switching between top-level tabs: no stack, no direction.
    Replace,
}

/// How one page replaces another. A page can ask for its own (`Route::enter_style`); otherwise the
/// interface's default applies.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TransitionStyle {
    /// Cut straight to the new page.
    None,
    /// Cross-fade.
    Fade,
    /// The new page slides in from the side it was opened from; the old one drifts away.
    Slide,
    /// The new page rises into place; going back sinks it.
    Rise,
    /// A small scale change with a fade.
    Zoom,
    /// The page fades up while it stages its own parts (banner zoom, artwork stagger) from the
    /// progress it reads with `use_page_motion`.
    Hero,
}

/// How much motion the user wants. One value per interface, so a handheld can be calm on the desktop
/// and lively on a TV, or the reverse.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Intensity {
    Off,
    Subtle,
    Standard,
    Cinematic,
}

impl Intensity {
    pub const ALL: [Intensity; 4] = [Self::Off, Self::Subtle, Self::Standard, Self::Cinematic];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Subtle => "Subtle",
            Self::Standard => "Standard",
            Self::Cinematic => "Cinematic",
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::Standard)
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|i| *i == self).unwrap_or(2)
    }

    /// Base length of a transition at this intensity.
    pub fn duration_ms(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Subtle => 160,
            Self::Standard => 240,
            Self::Cinematic => 420,
        }
    }

    /// How far things travel, in logical px.
    pub fn distance(self) -> f32 {
        match self {
            Self::Off => 0.,
            Self::Subtle => 24.,
            Self::Standard => 48.,
            Self::Cinematic => 96.,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Easing {
    /// Fast start, long settle: the default for things arriving.
    ExpoOut,
    CubicInOut,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intensity_indices_round_trip_and_unknown_falls_back() {
        for i in Intensity::ALL {
            assert_eq!(Intensity::from_index(i.index()), i);
        }
        assert_eq!(Intensity::from_index(99), Intensity::Standard);
    }

    #[test]
    fn stronger_intensity_means_longer_and_farther() {
        let steps: Vec<_> = Intensity::ALL.iter().map(|i| (i.duration_ms(), i.distance())).collect();
        assert!(steps.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1), "{steps:?}");
    }
}
