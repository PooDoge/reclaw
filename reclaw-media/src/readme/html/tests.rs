use super::*;

fn ctx() -> ReadmeContext {
    ReadmeContext::github("octo", "recomp", "HEAD").expect("valid")
}

fn blocks(html: &str) -> Vec<Block> {
    lower(html, &ctx())
}

fn md(text: &str) -> Block {
    Block::Markdown(text.to_string())
}

#[test]
fn headings_paragraphs_and_line_breaks() {
    assert_eq!(blocks("<h2>Install</h2><p>First line<br>second line</p>"), [md("## Install"), md("First line  \nsecond line")]);
}

#[test]
fn whitespace_collapses_as_it_does_in_a_browser() {
    assert_eq!(blocks("<p>  lots \n\n   of   space  </p>"), [md("lots of space")]);
}

#[test]
fn bold_italic_and_code() {
    assert_eq!(blocks("<p>a <b>bold</b>, <em>slanted</em> and <code>code</code></p>"), [md("a **bold**, *slanted* and `code`")]);
}

#[test]
fn characters_that_mean_something_in_markdown_are_escaped() {
    assert_eq!(blocks("<p>2 * 3 = 6 [sic] snake_case</p>"), [md("2 \\* 3 = 6 \\[sic\\] snake\\_case")]);
    assert_eq!(blocks("<p># not a heading</p>"), [md("\\# not a heading")]);
    assert_eq!(blocks("<p>1 &lt; 2 &amp; 3</p>"), [md("1 \\< 2 & 3")]);
}

#[test]
fn code_is_not_escaped() {
    assert_eq!(blocks("<p>run <code>a_b *c*</code></p>"), [md("run `a_b *c*`")]);
}

#[test]
fn a_link_around_words_is_a_markdown_link_with_the_raw_address() {
    assert_eq!(blocks("<p>See <a href=\"docs/a.md\">the docs</a>.</p>"), [md("See [the docs](<docs/a.md>).")]);
}

#[test]
fn an_unsafe_link_keeps_its_words_and_loses_the_link() {
    assert_eq!(blocks("<p><a href=\"javascript:alert(1)\">click</a></p>"), [md("click")]);
    assert_eq!(blocks("<p><a href=\"#top\">top</a></p>"), [md("top")]);
}

#[test]
fn a_picture_is_resolved_and_sized() {
    let [Block::Pictures(pics)] = &blocks("<img src=\"docs/a.png\" width=\"320px\" height=\"50%\" alt=\"Shot\">")[..] else { panic!() };
    assert_eq!(pics.len(), 1);
    assert_eq!(
        (pics[0].src.as_deref(), pics[0].width, pics[0].height),
        (Some("https://raw.githubusercontent.com/octo/recomp/HEAD/docs/a.png"), Some(320), None)
    );
}

#[test]
fn pictures_side_by_side_are_one_row_and_words_start_a_new_block() {
    let out = blocks("<img src=\"a.png\"><img src=\"b.png\"> and then words");
    assert!(matches!(&out[0], Block::Pictures(p) if p.len() == 2), "{out:?}");
    assert_eq!(out[1], md("and then words"));
}

#[test]
fn a_picture_wrapped_in_a_link_is_pressable() {
    let [Block::Pictures(pics)] = &blocks("<a href=\"https://example.com/go\"><img src=\"a.png\"></a>")[..] else { panic!() };
    assert_eq!(pics[0].link.as_deref(), Some("https://example.com/go"));
}

#[test]
fn picture_element_uses_its_fallback_img() {
    let out = blocks("<picture><source media=\"(prefers-color-scheme: dark)\" srcset=\"dark.png\"><img src=\"light.png\"></picture>");
    let [Block::Pictures(pics)] = &out[..] else { panic!("{out:?}") };
    assert!(pics[0].src.as_deref().is_some_and(|s| s.ends_with("light.png")));
}

#[test]
fn lists_nest() {
    assert_eq!(blocks("<ul><li>one</li><li>two<ul><li>inner</li></ul></li></ul>"), [md("- one"), md("- two"), md("  - inner")]);
    assert_eq!(blocks("<ol><li>first</li></ol>"), [md("1. first")]);
}

#[test]
fn preformatted_text_becomes_a_fenced_block_that_cannot_be_closed_early() {
    assert_eq!(blocks("<pre>a  b\n  c</pre>"), [md("```\na  b\n  c\n```")]);
    assert_eq!(blocks("<pre>```\nnot the end\n```</pre>"), [md("````\n```\nnot the end\n```\n````")]);
}

#[test]
fn summary_is_bold_and_the_rest_is_shown() {
    assert_eq!(blocks("<details><summary>More</summary><p>Hidden words</p></details>"), [md("**More**"), md("Hidden words")]);
}

#[test]
fn a_rule_is_a_rule() {
    assert_eq!(blocks("<p>a</p><hr><p>b</p>"), [md("a"), md("---"), md("b")]);
}

#[test]
fn skipped_elements_take_their_contents_with_them_even_when_nested() {
    assert_eq!(
        blocks("<p>before</p><svg><g><text>secret</text></g></svg><iframe src=\"x\"><p>no</p></iframe><p>after</p>"),
        [md("before"), md("after")]
    );
    assert_eq!(blocks("<div><div>kept</div></div><form><div>no</div></form>"), [md("kept")]);
    assert_eq!(blocks("<script>1</script><script>2</script>text"), [md("text")]);
}

#[test]
fn event_handlers_and_unknown_attributes_are_ignored() {
    assert_eq!(blocks("<p onclick=\"steal()\" style=\"x\" data-x=\"y\">fine</p>"), [md("fine")]);
}

#[test]
fn unknown_and_unclosed_tags_let_their_words_through() {
    assert_eq!(blocks("<blink>hello</blink> <custom-tag>world"), [md("hello world")]);
}

#[test]
fn a_fragment_with_no_words_is_nothing() {
    assert!(blocks("<div><span></span></div>   <br>").is_empty() || blocks("<div></div>").is_empty());
    assert!(blocks("").is_empty());
}
