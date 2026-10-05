//! The colours of a game's generated banner: what the game page shows when neither the catalog nor
//! the README has a picture wide enough. A project always gets the same hue (taken from its name, so
//! the library looks varied and a game does not change colour between runs), mixed into the theme's
//! own background so it follows light and dark without a branch.
//!
//! Nothing here knows about Freya: colours are plain `(r, g, b)` triples.

/// An RGB colour.
pub type Rgb = (u8, u8, u8);

/// FNV-1a, 64 bit. Not `DefaultHasher`: that one is free to change between Rust releases, and a
/// game's banner colour must not.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// A hue in degrees, `0..360`, the same for the same text.
pub fn hue_of(seed: &str) -> f32 {
    (fnv1a(seed.trim().to_lowercase().as_str()) % 360) as f32
}

/// HSL to RGB, with `h` in degrees and `s`, `l` in `0..=1`.
pub fn hsl(h: f32, s: f32, l: f32) -> Rgb {
    let (h, s, l) = (h.rem_euclid(360.), s.clamp(0., 1.), l.clamp(0., 1.));
    let c = (1. - (2. * l - 1.).abs()) * s;
    let x = c * (1. - ((h / 60.) % 2. - 1.).abs());
    let m = l - c / 2.;
    let (r, g, b) = match (h / 60.) as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let byte = |v: f32| ((v + m) * 255.).round().clamp(0., 255.) as u8;
    (byte(r), byte(g), byte(b))
}

/// `a` moved `amount` (`0..=1`) of the way to `b`.
pub fn mix(a: Rgb, b: Rgb, amount: f32) -> Rgb {
    let amount = amount.clamp(0., 1.);
    let channel = |from: u8, to: u8| (f32::from(from) + (f32::from(to) - f32::from(from)) * amount).round() as u8;
    (channel(a.0, b.0), channel(a.1, b.1), channel(a.2, b.2))
}

/// The two ends of the banner's gradient for a project: the hue's colour washed into `base` at the
/// top left, fading almost all the way back to `base` at the bottom right (where the title sits).
pub fn gradient(seed: &str, base: Rgb) -> (Rgb, Rgb) {
    let hue = hue_of(seed);
    let strong = mix(base, hsl(hue, 0.62, 0.46), 0.62);
    let faint = mix(base, hsl(hue + 38., 0.5, 0.4), 0.16);
    (strong, faint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_always_gets_the_same_hue_whatever_its_case_or_padding() {
        assert_eq!(hue_of("Starfall 64"), hue_of("  starfall 64 "));
        assert!((0.0..360.0).contains(&hue_of("Starfall 64")));
    }

    #[test]
    fn the_hue_does_not_depend_on_the_rust_release() {
        // Pinned: if this changes, every game's banner changes colour.
        assert_eq!(hue_of("zelda"), (fnv1a("zelda") % 360) as f32);
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn different_games_do_not_all_share_one_colour() {
        let names =
            ["Starfall 64", "Skyward Quest", "Kart Ruins", "Dino Rush", "Moon Garden", "Tide Racer", "Ship of Harkinian", "Mario 64"];
        let hues: std::collections::BTreeSet<u32> = names.iter().map(|n| hue_of(n) as u32).collect();
        assert!(hues.len() >= 6, "{hues:?}");
    }

    #[test]
    fn hsl_hits_the_primaries() {
        assert_eq!(hsl(0., 1., 0.5), (255, 0, 0));
        assert_eq!(hsl(120., 1., 0.5), (0, 255, 0));
        assert_eq!(hsl(240., 1., 0.5), (0, 0, 255));
        assert_eq!(hsl(0., 0., 1.), (255, 255, 255));
        assert_eq!(hsl(360., 1., 0.5), (255, 0, 0), "a full turn is the start");
        assert_eq!(hsl(-120., 1., 0.5), (0, 0, 255), "a negative hue wraps");
    }

    #[test]
    fn mixing_goes_from_one_colour_to_the_other() {
        assert_eq!(mix((0, 0, 0), (200, 100, 50), 0.), (0, 0, 0));
        assert_eq!(mix((0, 0, 0), (200, 100, 50), 1.), (200, 100, 50));
        assert_eq!(mix((0, 0, 0), (200, 100, 50), 0.5), (100, 50, 25));
        assert_eq!(mix((0, 0, 0), (200, 100, 50), 7.), (200, 100, 50), "clamped");
    }

    #[test]
    fn the_gradient_is_the_themes_own_background_toward_the_end() {
        let dark = (12, 14, 22);
        let (strong, faint) = gradient("Starfall 64", dark);
        let distance = |a: Rgb, b: Rgb| {
            i32::from(a.0).abs_diff(i32::from(b.0)) + i32::from(a.1).abs_diff(i32::from(b.1)) + i32::from(a.2).abs_diff(i32::from(b.2))
        };
        assert!(distance(strong, dark) > distance(faint, dark), "the far end is quieter");
        let light = (246, 247, 250);
        let (strong_light, faint_light) = gradient("Starfall 64", light);
        assert!(distance(strong_light, light) > distance(faint_light, light));
    }
}
