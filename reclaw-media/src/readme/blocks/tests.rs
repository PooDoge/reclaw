use super::*;
use crate::readme::Badge;

fn ctx() -> ReadmeContext {
    ReadmeContext::github("octo", "recomp", "HEAD").expect("valid")
}

fn doc(source: &str) -> Vec<Block> {
    parse(source, &ctx()).blocks
}

fn md(text: &str) -> Block {
    Block::Markdown(text.to_string())
}

const RAW: &str = "https://raw.githubusercontent.com/octo/recomp/HEAD/";

#[test]
fn plain_markdown_is_cut_into_its_top_level_blocks_and_nothing_is_changed() {
    let text = "# Title\n\nSome *words* and a list:\n\n- one\n- two\n\n```rust\nfn main() {}\n```\n";
    assert_eq!(doc(text), [md("# Title"), md("Some *words* and a list:"), md("- one\n- two"), md("```rust\nfn main() {}\n```")]);
}

#[test]
fn nothing_in_nothing_out() {
    for empty in ["", "   \n\n  ", "\u{feff}"] {
        assert!(parse(empty, &ctx()).is_empty(), "{empty:?}");
    }
}

#[test]
fn a_picture_on_its_own_line_is_lifted_out_of_the_prose() {
    let blocks = doc("Intro\n\n![Menu](docs/menu.png)\n\nOutro\n");
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0], md("Intro"));
    let Block::Pictures(pics) = &blocks[1] else { panic!("{blocks:?}") };
    assert_eq!((pics[0].src.as_deref(), pics[0].alt.as_str()), (Some(format!("{RAW}docs/menu.png").as_str()), "Menu"));
    assert_eq!(blocks[2], md("Outro"));
}

#[test]
fn the_start_of_a_readme_is_measured_in_reading_not_in_blocks() {
    let long: String =
        (1..=30).map(|i| format!("## Section {i}\n\nA paragraph of words for section {i}, long enough to count.\n\n")).collect();
    let document = parse(&long, &ctx());
    let n = document.preview_len(600);
    assert!(n >= 2 && n < document.blocks.len(), "a start, not everything: {n} of {}", document.blocks.len());
    assert_eq!(document.preview_len(usize::MAX), document.blocks.len());
    assert_eq!(document.preview_len(0), 0, "no budget, nothing; the caller shows at least what it chooses");
    let heavy = parse("![a](a.png)\n\n![b](b.png)\n\n![c](c.png)\n", &ctx());
    assert_eq!(heavy.preview_len(500), 2, "pictures weigh something too");
}

#[test]
fn a_badge_in_a_link_is_a_pressable_badge() {
    let blocks = doc("[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://ci.example.com/runs)\n");
    let Block::Pictures(pics) = &blocks[0] else { panic!("{blocks:?}") };
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0].link.as_deref(), Some("https://ci.example.com/runs"));
    let badge: &Badge = pics[0].badge.as_ref().expect("a badge");
    assert_eq!((badge.label.as_deref(), badge.message.as_str()), (Some("build"), "passing"));
}

#[test]
fn a_row_of_badges_is_one_row() {
    let text = "[![a](https://img.shields.io/badge/a-1-blue)](https://a.example.com) [![b](https://img.shields.io/badge/b-2-red)](https://b.example.com)\n";
    let Block::Pictures(pics) = &doc(text)[0] else { panic!() };
    assert_eq!(
        pics.iter().map(|p| p.link.as_deref().unwrap_or_default()).collect::<Vec<_>>(),
        ["https://a.example.com/", "https://b.example.com/"]
    );
}

#[test]
fn a_picture_among_words_is_not_lifted_and_becomes_a_link() {
    assert_eq!(doc("See ![the menu](docs/menu.png) for details.\n"), [md(&format!("See [the menu]({RAW}docs/menu.png) for details."))]);
}

#[test]
fn a_centered_html_header_becomes_a_logo_a_heading_and_badges() {
    let html = "<p align=\"center\">\n  <img src=\"docs/logo.png\" width=\"200\" alt=\"Logo\">\n</p>\n\n<h1 align=\"center\">Starfall Recomp</h1>\n\n<p align=\"center\">\n  <a href=\"https://ci.example.com\"><img src=\"https://img.shields.io/badge/build-passing-green\" alt=\"build\"></a>\n  <a href=\"LICENSE\"><img src=\"https://img.shields.io/github/license/octo/recomp\" alt=\"License: MIT\"></a>\n</p>\n\nA static recompilation.\n";
    let blocks = doc(html);
    assert_eq!(blocks.len(), 4, "{blocks:#?}");
    let Block::Pictures(logo) = &blocks[0] else { panic!("{blocks:?}") };
    assert_eq!(
        (logo[0].src.as_deref(), logo[0].width, logo[0].badge.is_none()),
        (Some(format!("{RAW}docs/logo.png").as_str()), Some(200), true)
    );
    assert_eq!(blocks[1], md("# Starfall Recomp"));
    let Block::Pictures(badges) = &blocks[2] else { panic!("{blocks:?}") };
    assert_eq!(badges.len(), 2);
    assert_eq!(badges[0].link.as_deref(), Some("https://ci.example.com/"));
    assert_eq!(badges[1].link.as_deref(), Some("https://github.com/octo/recomp/blob/HEAD/LICENSE"));
    assert_eq!(badges[1].badge.as_ref().map(|b| b.message.as_str()), Some("License: MIT"));
    assert_eq!(blocks[3], md("A static recompilation."));
}

#[test]
fn details_shows_its_summary_and_its_content() {
    let html = "<details>\n<summary>Build from source</summary>\n\nRun `make`.\n\n</details>\n";
    let text = doc(html);
    let joined: String =
        text.iter().filter_map(|b| if let Block::Markdown(m) = b { Some(m.as_str()) } else { None }).collect::<Vec<_>>().join("\n");
    assert!(joined.contains("**Build from source**"), "{text:?}");
    assert!(joined.contains("Run `make`."), "{text:?}");
}

#[test]
fn scripts_and_styles_are_dropped_with_their_contents() {
    let blocks = doc("<script>alert('x')</script>\n\n<style>p { color: red }</style>\n\n<div>kept</div>\n");
    assert_eq!(blocks, [md("kept")]);
}

#[test]
fn html_in_a_paragraph_is_dropped_but_its_words_stay() {
    assert_eq!(doc("Press <kbd>Enter</kbd> now <span onclick=\"x()\">please</span>\n"), [md("Press Enter now please")]);
}

#[test]
fn a_mermaid_fence_is_a_diagram_and_other_fences_are_code() {
    let blocks = doc("```mermaid\ngraph TD\n  A-->B\n```\n\n```rust\nlet x = 1;\n```\n");
    assert_eq!(blocks[0], Block::Diagram { language: "mermaid".into(), source: "graph TD\n  A-->B\n".into() });
    assert_eq!(blocks[1], md("```rust\nlet x = 1;\n```"));
    assert!(matches!(&doc("``` Mermaid\nflowchart\n```")[0], Block::Diagram { .. }), "the name is case-insensitive");
}

#[test]
fn a_fence_inside_a_list_is_not_cut_out_of_it() {
    let text = "1. Install:\n\n   ```mermaid\n   graph TD\n   ```\n\n2. Run it\n";
    assert_eq!(doc(text).len(), 1, "lists stay whole");
}

#[test]
fn reference_links_keep_their_definitions_made_absolute() {
    let blocks = doc("Read the [guide][g] and the [wiki].\n\n[g]: docs/GUIDE.md\n[wiki]: https://example.com/wiki\n");
    let Block::Markdown(text) = &blocks[0] else { panic!("{blocks:?}") };
    assert!(text.contains("[g]: <https://github.com/octo/recomp/blob/HEAD/docs/GUIDE.md>"), "{text}");
    assert!(text.contains("[wiki]: <https://example.com/wiki>"), "{text}");
}

#[test]
fn a_definition_nobody_uses_is_not_written_out() {
    let Block::Markdown(text) = &doc("Just words.\n\n[unused]: docs/x.md\n")[0] else { panic!() };
    assert!(!text.contains("unused"), "{text}");
}

#[test]
fn a_reference_style_picture_is_a_picture() {
    let blocks = doc("![Shot][s]\n\n[s]: docs/shot.png\n");
    let Block::Pictures(pics) = &blocks[0] else { panic!("{blocks:?}") };
    assert_eq!(pics[0].src.as_deref(), Some(format!("{RAW}docs/shot.png").as_str()));
}

#[test]
fn a_table_with_a_picture_in_a_cell_keeps_its_shape_and_loses_the_picture() {
    let blocks = doc("| a | b |\n|---|---|\n| ![x](p.png) | 2 |\n");
    assert_eq!(blocks, [md(&format!("| a | b |\n|---|---|\n| [x]({RAW}p.png) | 2 |"))]);
}

#[test]
fn a_picture_with_an_unusable_address_still_shows_its_alt_text() {
    let blocks = doc("![Chart](data:image/png;base64,AAAA)\n");
    let Block::Pictures(pics) = &blocks[0] else { panic!("{blocks:?}") };
    assert_eq!((pics[0].src.clone(), pics[0].alt.as_str()), (None, "Chart"));
}

#[test]
fn hostile_and_broken_input_is_survived() {
    for text in [
        "<<<<<>>>>>",
        "<img src=",
        "<a href=\"javascript:alert(1)\"><img src=\"x.png\"></a>",
        "[x](",
        "![](",
        "```\nunclosed",
        &"<div>".repeat(2000),
        &"> ".repeat(500),
        "\0\0\0",
    ] {
        let _ = parse(text, &ctx());
    }
    let Block::Pictures(pics) = &doc("<a href=\"javascript:alert(1)\"><img src=\"x.png\"></a>")[0] else { panic!() };
    assert_eq!(pics[0].link, None, "a javascript: link is not carried");
}

#[test]
fn a_realistic_readme_comes_out_in_reading_order() {
    let text = "<div align=\"center\"><img src=\"logo.svg\" width=\"120\"></div>\n\n# Kart Recomp\n\n[![CI](https://github.com/octo/recomp/actions/workflows/ci.yml/badge.svg)](https://github.com/octo/recomp/actions)\n\nPlay the classic natively.\n\n## Screenshots\n\n![Race](docs/race.png)\n![Menu](docs/menu.png)\n\n## Building\n\n```sh\ncargo build --release\n```\n";
    let mut kinds: Vec<&str> = doc(text)
        .iter()
        .map(|b| match b {
            Block::Markdown(_) => "md",
            Block::Pictures(_) => "pics",
            Block::Diagram { .. } => "diagram",
        })
        .collect();
    kinds.dedup();
    assert_eq!(kinds, ["pics", "md", "pics", "md", "pics", "md"], "logo, title, badge, prose and headings, two screenshots, build section");
}

/// A README is written by a stranger, so the whole pipeline must survive anything. This throws
/// thousands of scrambled fragments of markup at it; it checks that nothing panics (a slice cut
/// inside a character, an index past the end), not what comes out.
#[test]
fn scrambled_markup_never_panics() {
    const PIECES: &[&str] = &[
        "<",
        ">",
        "/",
        "=",
        "\"",
        "'",
        "&",
        ";",
        "#",
        "x",
        " ",
        "\n",
        "é",
        "—",
        "\u{feff}",
        "<a ",
        "<img src=",
        "</",
        "<!--",
        "-->",
        "<p>",
        "</p>",
        "<details>",
        "<summary>",
        "<pre>",
        "</pre>",
        "<script>",
        "</script",
        "<svg>",
        "[",
        "](",
        "![",
        ")",
        "`",
        "```",
        "~~~",
        "*",
        "_",
        "|",
        "---",
        "1. ",
        "- ",
        "> ",
        "\\",
        "&amp;",
        "&#x41;",
        "&#0;",
        "https://",
        "javascript:",
        "mailto:",
        "data:",
        "..",
        "//",
        "mermaid",
    ];
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let ctx = ctx();
    for _ in 0..3000 {
        let len = (next() % 40) as usize;
        let text: String = (0..len).map(|_| PIECES[(next() % PIECES.len() as u64) as usize]).collect();
        let _ = parse(&text, &ctx);
    }
}
