//! Token values, generated from design-system/tokens.json. Regenerate, do not hand-edit.
use freya::prelude::*;

/// Every Reclaw token, including the ones ColorsSheet has no slot for (install green,
/// status fills, bg_deep). Read it with `use_reclaw()`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reclaw {
    pub bg_deep: Color,
    pub bg_nav: Color,
    pub bg_base: Color,
    pub bg_panel: Color,
    pub bg_raised: Color,
    pub line: Color,
    pub line_strong: Color,
    pub ink: Color,
    pub ink_muted: Color,
    pub ink_subtle: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub on_accent: Color,
    pub install: Color,
    pub install_hover: Color,
    pub on_install: Color,
    pub ok: Color,
    pub ok_bg: Color,
    pub warn: Color,
    pub warn_bg: Color,
    pub danger: Color,
    pub danger_bg: Color,
    pub info: Color,
    pub info_bg: Color,
    pub scrim: Color,
}

pub fn midnight() -> Reclaw {
    Reclaw {
        bg_deep: Color::from_rgb(11, 16, 23),
        bg_nav: Color::from_rgb(16, 24, 32),
        bg_base: Color::from_rgb(22, 32, 43),
        bg_panel: Color::from_rgb(28, 41, 55),
        bg_raised: Color::from_rgb(38, 55, 74),
        line: Color::from_rgb(44, 62, 82),
        line_strong: Color::from_rgb(91, 115, 144),
        ink: Color::from_rgb(232, 238, 245),
        ink_muted: Color::from_rgb(169, 184, 201),
        ink_subtle: Color::from_rgb(132, 150, 170),
        accent: Color::from_rgb(102, 192, 244),
        accent_hover: Color::from_rgb(143, 210, 248),
        on_accent: Color::from_rgb(8, 20, 31),
        install: Color::from_rgb(55, 121, 15),
        install_hover: Color::from_rgb(47, 106, 11),
        on_install: Color::from_rgb(255, 255, 255),
        ok: Color::from_rgb(134, 204, 74),
        ok_bg: Color::from_rgb(31, 51, 24),
        warn: Color::from_rgb(243, 189, 82),
        warn_bg: Color::from_rgb(58, 47, 18),
        danger: Color::from_rgb(255, 132, 132),
        danger_bg: Color::from_rgb(61, 28, 31),
        info: Color::from_rgb(102, 192, 244),
        info_bg: Color::from_rgb(18, 48, 63),
        scrim: Color::from_argb(179, 5, 8, 12),
    }
}

pub fn daylight() -> Reclaw {
    Reclaw {
        bg_deep: Color::from_rgb(221, 229, 238),
        bg_nav: Color::from_rgb(232, 238, 244),
        bg_base: Color::from_rgb(242, 245, 249),
        bg_panel: Color::from_rgb(255, 255, 255),
        bg_raised: Color::from_rgb(227, 234, 242),
        line: Color::from_rgb(207, 217, 228),
        line_strong: Color::from_rgb(111, 130, 151),
        ink: Color::from_rgb(20, 32, 44),
        ink_muted: Color::from_rgb(68, 85, 104),
        ink_subtle: Color::from_rgb(88, 105, 124),
        accent: Color::from_rgb(11, 106, 168),
        accent_hover: Color::from_rgb(10, 90, 144),
        on_accent: Color::from_rgb(255, 255, 255),
        install: Color::from_rgb(47, 122, 18),
        install_hover: Color::from_rgb(39, 106, 15),
        on_install: Color::from_rgb(255, 255, 255),
        ok: Color::from_rgb(45, 122, 20),
        ok_bg: Color::from_rgb(225, 241, 214),
        warn: Color::from_rgb(122, 79, 0),
        warn_bg: Color::from_rgb(251, 239, 207),
        danger: Color::from_rgb(179, 38, 30),
        danger_bg: Color::from_rgb(251, 220, 218),
        info: Color::from_rgb(11, 106, 168),
        info_bg: Color::from_rgb(216, 236, 248),
        scrim: Color::from_argb(166, 11, 16, 23),
    }
}

pub(crate) fn sheet(t: &Reclaw, base: ColorsSheet) -> ColorsSheet {
    ColorsSheet {
        primary: t.accent,
        secondary: t.install,
        tertiary: t.accent_hover,
        success: t.ok,
        warning: t.warn,
        error: t.danger,
        info: t.info,
        background: t.bg_base,
        surface_primary: t.bg_panel,
        surface_secondary: t.bg_raised,
        surface_tertiary: t.bg_nav,
        border: t.line_strong,
        border_focus: t.accent,
        text_primary: t.ink,
        text_secondary: t.ink_muted,
        text_placeholder: t.ink_subtle,
        text_inverse: t.on_accent,
        overlay: t.scrim,
        ..base
    }
}

pub fn reclaw_midnight() -> Theme {
    let mut theme = dark_theme();
    theme.name = "reclaw-midnight";
    theme.colors = sheet(&midnight(), DARK_COLORS);
    theme
}

pub fn reclaw_daylight() -> Theme {
    let mut theme = light_theme();
    theme.name = "reclaw-daylight";
    theme.colors = sheet(&daylight(), LIGHT_COLORS);
    theme
}
