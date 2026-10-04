//! Non-color tokens from tokens.json (spacing, radius, layout) plus the two responsive switches.

pub const SPACE_1: f32 = 4.;
pub const SPACE_2: f32 = 8.;
pub const SPACE_3: f32 = 12.;
pub const SPACE_4: f32 = 16.;
pub const SPACE_5: f32 = 24.;
pub const SPACE_6: f32 = 32.;
pub const SPACE_8: f32 = 48.;

pub const RADIUS_SM: f32 = 2.;
pub const RADIUS_MD: f32 = 4.;
pub const RADIUS_LG: f32 = 8.;

pub const SIDEBAR_W: f32 = 240.;
pub const RAIL_W: f32 = 64.;
pub const TOPBAR_H: f32 = 40.;
pub const TABBAR_H: f32 = 56.;
pub const ROW_H: f32 = 32.;
pub const ROW_H_TOUCH: f32 = 48.;
pub const TARGET_MIN: f32 = 44.;
pub const CAPSULE_W: f32 = 168.;
pub const CAPSULE_H: f32 = 224.;
pub const HERO_H: f32 = 280.;

pub const BP_WIDE: f32 = 1100.;
pub const BP_COMPACT: f32 = 720.;

/// Layout class, chosen from the root container width, never from the OS.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LayoutClass {
    #[default]
    Wide,
    Compact,
    Phone,
}

impl LayoutClass {
    pub fn from_width(width: f32) -> Self {
        if width >= BP_WIDE {
            Self::Wide
        } else if width >= BP_COMPACT {
            Self::Compact
        } else {
            Self::Phone
        }
    }

    /// Default density for the class. A handheld at wide width should override to `Touch`.
    pub fn default_density(self) -> Density {
        match self {
            Self::Phone => Density::Touch,
            _ => Density::Pointer,
        }
    }
}

/// Input-device density, independent of width: a 1280x800 handheld is `Wide` + `Touch`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Density {
    #[default]
    Pointer,
    Touch,
}

impl Density {
    pub fn row_height(self) -> f32 {
        match self {
            Self::Pointer => ROW_H,
            Self::Touch => ROW_H_TOUCH,
        }
    }

    pub fn button_height(self) -> f32 {
        match self {
            Self::Pointer => 32.,
            Self::Touch => 48.,
        }
    }

    pub fn body_size(self) -> f32 {
        match self {
            Self::Pointer => 14.,
            Self::Touch => 16.,
        }
    }
}
