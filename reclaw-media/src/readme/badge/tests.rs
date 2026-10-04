use super::*;

fn badge(src: &str, alt: &str) -> Badge {
    Badge::from_image(src, alt).unwrap_or_else(|| panic!("{src} should be a badge"))
}

#[test]
fn a_static_shields_badge_is_read_from_its_address() {
    let b = badge("https://img.shields.io/badge/build-passing-brightgreen.svg", "");
    assert_eq!((b.label.as_deref(), b.message.as_str()), (Some("build"), "passing"));
    assert_eq!(b.color, BadgeColor::new(0x44, 0xcc, 0x11));
}

#[test]
fn escapes_in_the_words_are_undone() {
    let b = badge("https://img.shields.io/badge/made_with-rust--lang_2024-orange", "");
    assert_eq!((b.label.as_deref(), b.message.as_str()), (Some("made with"), "rust-lang 2024"));
    let b = badge("https://img.shields.io/badge/snake__case-a%20b-blue", "");
    assert_eq!((b.label.as_deref(), b.message.as_str()), (Some("snake_case"), "a b"));
}

#[test]
fn a_badge_with_no_label_is_just_its_message() {
    let b = badge("https://img.shields.io/badge/just%20words-8A2BE2", "");
    assert_eq!((b.label, b.message.as_str()), (None, "just words"));
    assert_eq!(b.color, BadgeColor::new(0x8a, 0x2b, 0xe2));
}

#[test]
fn hex_colors_come_in_three_and_six_digits_with_or_without_a_hash() {
    assert_eq!(BadgeColor::parse("4c1"), Some(BadgeColor::new(0x44, 0xcc, 0x11)));
    assert_eq!(BadgeColor::parse("#FF8800"), Some(BadgeColor::new(255, 136, 0)));
    for bad in ["12", "gggggg", "", "1234567", "chartreuse"] {
        assert_eq!(BadgeColor::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn query_parameters_override_the_path() {
    let b = badge("https://img.shields.io/badge/a-b-red?label=Status&color=green", "");
    assert_eq!((b.label.as_deref(), b.message.as_str()), (Some("Status"), "b"));
    assert_eq!(b.color, BadgeColor::parse("green").expect("named"));
}

#[test]
fn a_badge_whose_words_are_not_in_the_address_says_its_alt_text() {
    let b = badge("https://img.shields.io/github/license/octo/recomp", "License: MIT");
    assert_eq!((b.label, b.message.as_str()), (None, "License: MIT"));
    let b = badge("https://img.shields.io/github/stars/octo/recomp", "");
    assert_eq!(b.message, "img.shields.io", "with no alt text, at least where it is from");
}

#[test]
fn other_badge_sites_are_recognized() {
    for src in [
        "https://travis-ci.org/octo/recomp.svg?branch=main",
        "https://codecov.io/gh/octo/recomp/branch/main/graph/badge.svg",
        "https://github.com/octo/recomp/actions/workflows/ci.yml/badge.svg",
        "https://gitlab.com/octo/recomp/badges/main/pipeline.svg",
    ] {
        assert!(Badge::from_image(src, "CI").is_some(), "{src}");
    }
}

#[test]
fn ordinary_pictures_are_not_badges() {
    for src in [
        "https://example.com/screenshot.png",
        "https://raw.githubusercontent.com/octo/recomp/HEAD/logo.svg",
        "not a url",
        "https://example.com/badges.png",
    ] {
        assert_eq!(Badge::from_image(src, ""), None, "{src}");
    }
}

#[test]
fn text_color_follows_the_background() {
    assert!(BadgeColor::parse("yellow").expect("named").wants_dark_text());
    assert!(!BadgeColor::parse("blue").expect("named").wants_dark_text());
    assert!(!BadgeColor::parse("red").expect("named").wants_dark_text());
    assert!(BadgeColor::parse("lightgrey").expect("named").wants_dark_text(), "the neutral grey is light enough for dark text");
    assert!(!BadgeColor::parse("333").expect("hex").wants_dark_text());
}
