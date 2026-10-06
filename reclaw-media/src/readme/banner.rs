//! Which pictures in a README could stand in for a game's banner when the catalog has none.
//!
//! Most catalog entries carry only an icon, so the game page's wide banner would stay empty. A README
//! usually opens with something wide: a title card, a screenshot, a logo on a background. This picks
//! the likely ones, best first, from the text alone. Nothing is fetched here; the UI tries them in
//! order and keeps the first whose real size is wide enough ([`fits_banner`]), because a README states
//! a size for only some pictures.
use super::{Block, Document, Pic};

/// A banner is cropped to about 3:1; narrower than this and a crop shows only the middle of the picture.
pub const MIN_ASPECT: f32 = 1.6;
/// Narrower than this in pixels and it is an icon or a button, which would be blown up into mush.
pub const MIN_WIDTH: u32 = 480;

/// Whether a picture of this size (when the file's header could be read) makes a banner. An unknown
/// size is not accepted: showing a wrong crop is worse than showing the generated banner.
pub fn fits_banner(size: Option<(u32, u32)>) -> bool {
    size.is_some_and(|(w, h)| h > 0 && w >= MIN_WIDTH && w as f32 / h as f32 >= MIN_ASPECT)
}

/// Words in a picture's address or alt text that mark what it is. The score is how likely a banner it is.
const WANTED: &[(&str, i32)] = &[
    ("banner", 6),
    ("header", 6),
    ("hero", 6),
    ("cover", 5),
    ("splash", 5),
    ("title", 4),
    ("screenshot", 4),
    ("preview", 4),
    ("gameplay", 4),
    ("showcase", 4),
    ("logo", 1),
];

/// Words that mark a button, a badge or a face: never a banner, whatever size it is.
const UNWANTED: &[&str] = &[
    "badge",
    "shields.io",
    "badgen",
    "button",
    "donate",
    "sponsor",
    "patreon",
    "ko-fi",
    "kofi",
    "buymeacoffee",
    "paypal",
    "discord",
    "avatar",
    "emoji",
    "workflows",
    "codecov",
    "travis",
    "codacy",
    "opencollective",
    "icon",
    "favicon",
];

fn score(pic: &Pic, position: usize) -> Option<i32> {
    let url = pic.src.as_deref()?;
    if pic.badge.is_some() {
        return None;
    }
    let haystack = format!("{} {}", url.to_ascii_lowercase(), pic.alt.to_ascii_lowercase());
    if UNWANTED.iter().any(|word| haystack.contains(word)) {
        return None;
    }
    // A size the README states settles it without a fetch.
    if let Some(w) = pic.width
        && w < MIN_WIDTH
    {
        return None;
    }
    if let (Some(w), Some(h)) = (pic.width, pic.height)
        && (h == 0 || (w as f32 / h as f32) < MIN_ASPECT)
    {
        return None;
    }
    let hints: i32 = WANTED.iter().filter(|(word, _)| haystack.contains(word)).map(|(_, points)| *points).max().unwrap_or(0);
    // A GIF is often a recording: heavy to fetch for a banner, so it ranks behind a still.
    let motion = if url.to_ascii_lowercase().split(['?', '#']).next().is_some_and(|p| p.ends_with(".gif")) { 3 } else { 0 };
    // The start of a README is where the title card is.
    let early = 3_i32.saturating_sub(position as i32);
    Some(hints + early - motion)
}

/// Up to `limit` addresses to try as the banner, most promising first (ties keep the README's order).
pub fn banner_candidates(doc: &Document, limit: usize) -> Vec<String> {
    let mut scored: Vec<(i32, usize, String)> = Vec::new();
    let mut position = 0;
    for block in &doc.blocks {
        if let Block::Pictures(pics) = block {
            for pic in pics {
                if let (Some(points), Some(url)) = (score(pic, position), pic.src.as_ref())
                    && !scored.iter().any(|(_, _, seen)| seen == url)
                {
                    scored.push((points, position, url.clone()));
                }
                position += 1;
            }
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().take(limit).map(|(_, _, url)| url).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readme::{ReadmeContext, parse};

    fn candidates(markdown: &str) -> Vec<String> {
        let context = ReadmeContext::github("owner", "game", "HEAD").expect("a valid repository");
        banner_candidates(&parse(markdown, &context), 3)
    }

    const RAW: &str = "https://raw.githubusercontent.com/owner/game/HEAD/";

    #[test]
    fn a_title_card_beats_a_screenshot_further_down() {
        let found = candidates("![Banner](docs/banner.png)\n\nSome text.\n\n![shot](docs/shot.png)\n");
        assert_eq!(found, vec![format!("{RAW}docs/banner.png"), format!("{RAW}docs/shot.png")]);
    }

    #[test]
    fn badges_buttons_and_sponsor_links_are_never_candidates() {
        let found = candidates(
            "[![Build](https://img.shields.io/github/actions/workflow/status/o/g/ci.yml)](https://x.test)\n\n\
             ![Donate](https://img.shields.io/badge/donate-paypal-blue.svg)\n\n\
             ![Join us on Discord](assets/discord.png)\n\n![the game](assets/preview.png)\n",
        );
        assert_eq!(found, vec![format!("{RAW}assets/preview.png")]);
    }

    #[test]
    fn a_stated_size_that_cannot_be_a_banner_rules_a_picture_out_without_fetching_it() {
        let found = candidates(
            "<img src=\"assets/logo.png\" width=\"128\" height=\"128\">\n\n\
             <img src=\"assets/square.png\" width=\"800\" height=\"800\">\n\n\
             <img src=\"assets/wide.png\" width=\"1200\" height=\"400\">\n",
        );
        assert_eq!(found, vec![format!("{RAW}assets/wide.png")]);
    }

    #[test]
    fn a_picture_listed_twice_is_listed_once() {
        let found = candidates("![Banner](banner.png)\n\n![Banner again](banner.png)\n");
        assert_eq!(found, vec![format!("{RAW}banner.png")]);
    }

    #[test]
    fn a_recording_ranks_behind_a_still() {
        let found = candidates("![demo](assets/demo.gif)\n\n![screenshot](assets/shot.png)\n");
        assert_eq!(found.first().map(String::as_str), Some(format!("{RAW}assets/shot.png").as_str()));
    }

    #[test]
    fn a_readme_with_no_pictures_has_no_candidates() {
        assert!(candidates("# Game\n\nJust words.\n").is_empty());
    }

    #[test]
    fn the_limit_is_respected() {
        let md = "![a](a.png)\n\n![b](b.png)\n\n![c](c.png)\n\n![d](d.png)\n";
        let context = ReadmeContext::github("o", "g", "HEAD").expect("valid");
        assert_eq!(banner_candidates(&parse(md, &context), 2).len(), 2);
    }

    #[test]
    fn only_wide_and_big_enough_pictures_fit() {
        assert!(fits_banner(Some((1200, 600))));
        assert!(fits_banner(Some((1920, 620))));
        assert!(!fits_banner(Some((512, 512))), "square");
        assert!(!fits_banner(Some((300, 100))), "too small");
        assert!(!fits_banner(Some((900, 900))), "square and large");
        assert!(!fits_banner(None), "unknown size");
        assert!(!fits_banner(Some((800, 0))));
    }
}
