//! Deck mode follows the theme: its page, the drawer over it, and what it dims. Read from real pixels,
//! because the failures were visible and not logical: a dark page behind dark text in daylight, and a
//! scrim painted over the drawer's own background.

use freya::prelude::NamedKey;
use reclaw_ui::theme::ThemeKind;

use crate::common::*;

const THEMES: [ThemeKind; 2] = [ThemeKind::Midnight, ThemeKind::Daylight];

/// Relative brightness, 0 to 1, close enough to tell a light page from a dark one.
fn brightness((r, g, b): (u8, u8, u8)) -> f32 {
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.
}

fn rgb(c: freya::prelude::Color) -> (u8, u8, u8) {
    (c.r(), c.g(), c.b())
}

#[test]
fn the_deck_page_is_light_in_daylight_and_dark_in_midnight() {
    let mut dark = Mount::deck().theme(ThemeKind::Midnight).size(1280., 800.).start();
    let mut light = Mount::deck().theme(ThemeKind::Daylight).size(1280., 800.).start();
    // The top right of Home is empty page in both.
    let (dark, light) = (brightness(dark.pixel(1200, 100)), brightness(light.pixel(1200, 100)));
    assert!(dark < 0.15, "midnight page brightness {dark}");
    assert!(light > 0.7, "daylight page brightness {light}: the page stayed dark behind dark text");
}

#[test]
fn the_main_menu_drawer_keeps_its_own_background_and_the_page_behind_it_dims() {
    for theme in THEMES {
        let mut s = Mount::deck().theme(theme).size(1280., 800.).start();
        let page_before = brightness(s.pixel(1100, 400));
        s.press(NamedKey::Tab);
        s.pump(400);
        // Below the last row the drawer shows nothing but its background.
        assert_eq!(s.pixel(200, 700), rgb(theme.tokens().bg_panel), "{theme:?}: the drawer was dimmed by its scrim");
        let page_after = brightness(s.pixel(1100, 400));
        assert!(page_after < page_before * 0.8, "{theme:?}: the page behind did not dim ({page_before} to {page_after})");
    }
}

fn luminance(c: freya::prelude::Color) -> f32 {
    let linear = |v: u8| {
        let v = v as f32 / 255.;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}

fn contrast(a: freya::prelude::Color, b: freya::prelude::Color) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The focused row of a menu is drawn as `ink` with `deck-bg` text, and the row a submenu came from as `ink-muted`
/// with `deck-bg` text. When `deck-bg` stayed near-black in daylight the first was dark text on a dark highlight.
#[test]
fn the_colours_a_menu_row_is_drawn_in_are_readable_in_both_themes() {
    for theme in THEMES {
        let t = theme.tokens();
        assert!(contrast(t.ink, t.deck_bg) >= 4.5, "{theme:?}: the focused row, {}", contrast(t.ink, t.deck_bg));
        assert!(contrast(t.ink_muted, t.deck_bg) >= 4.5, "{theme:?}: the row a submenu came from, {}", contrast(t.ink_muted, t.deck_bg));
        assert!(contrast(t.ink, t.bg_panel) >= 4.5, "{theme:?}: an ordinary row on the panel");
    }
}
