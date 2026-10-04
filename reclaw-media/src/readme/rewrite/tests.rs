use super::*;

fn ctx() -> ReadmeContext {
    ReadmeContext::github("octo", "recomp", "HEAD").expect("valid")
}

fn fix(source: &str) -> String {
    markdown(source, &ctx())
}

#[test]
fn a_relative_link_becomes_an_absolute_one_and_the_rest_is_untouched() {
    assert_eq!(
        fix("See [the guide](docs/BUILDING.md) first.\n"),
        "See [the guide](https://github.com/octo/recomp/blob/HEAD/docs/BUILDING.md) first.\n"
    );
    assert_eq!(
        fix("[a](https://example.com/x) and *emphasis*"),
        "[a](https://example.com/x) and *emphasis*",
        "nothing to change, nothing changed"
    );
}

#[test]
fn a_link_with_a_title_or_angle_brackets_keeps_them() {
    assert_eq!(fix("[x](docs/a.md \"A title\")"), "[x](https://github.com/octo/recomp/blob/HEAD/docs/a.md \"A title\")");
    assert_eq!(fix("[x](<docs/a b.md>)"), "[x](<https://github.com/octo/recomp/blob/HEAD/docs/a%20b.md>)");
}

#[test]
fn links_that_cannot_be_opened_safely_become_their_words() {
    assert_eq!(fix("[click](javascript:alert(1))"), "click");
    assert_eq!(fix("Mail [me](mailto:a@b.c)."), "Mail me.");
    assert_eq!(fix("Jump to [**Install**](#install) now"), "Jump to **Install** now");
}

#[test]
fn an_image_in_prose_becomes_a_link_to_it() {
    assert_eq!(
        fix("Look: ![the menu](docs/menu.png) nice."),
        "Look: [the menu](https://raw.githubusercontent.com/octo/recomp/HEAD/docs/menu.png) nice."
    );
    assert_eq!(fix("![](a.png)"), "[image](https://raw.githubusercontent.com/octo/recomp/HEAD/a.png)", "no alt text: a word for it");
}

#[test]
fn an_image_inside_a_link_keeps_the_link_and_gives_up_the_picture() {
    assert_eq!(
        fix("[![build](https://img.shields.io/badge/a-b-green)](https://ci.example.com/run) status"),
        "[build](https://ci.example.com/run) status"
    );
}

#[test]
fn an_image_that_cannot_be_fetched_is_only_its_words() {
    assert_eq!(fix("A ![chart](data:image/png;base64,AAAA) here"), "A chart here");
}

#[test]
fn square_brackets_in_alt_text_cannot_break_the_link() {
    assert_eq!(fix("x ![a [b] c](p.png) y"), "x [a \\[b\\] c](https://raw.githubusercontent.com/octo/recomp/HEAD/p.png) y");
}

#[test]
fn raw_html_in_a_paragraph_goes_but_a_line_break_stays() {
    assert_eq!(fix("one<br>two <b>bold</b> <img src=\"x.png\">end"), "one  \ntwo bold end");
    assert_eq!(fix("a<br/>b"), "a  \nb");
}

#[test]
fn code_is_never_touched() {
    let code = "```\n[x](docs/a.md) ![i](p.png) <br>\n```\n\nand `![i](p.png)` inline\n";
    assert_eq!(fix(code), code);
}

#[test]
fn several_edits_in_one_text_do_not_disturb_each_other() {
    let out = fix("[a](x.md) ![i](p.png) [b](#top) [c](https://example.com)");
    assert_eq!(
        out,
        "[a](https://github.com/octo/recomp/blob/HEAD/x.md) [i](https://raw.githubusercontent.com/octo/recomp/HEAD/p.png) b [c](https://example.com)"
    );
}

#[test]
fn tables_keep_their_shape() {
    let table = "| a | b |\n|---|---|\n| [x](docs/x.md) | 2 |\n";
    assert_eq!(fix(table), "| a | b |\n|---|---|\n| [x](https://github.com/octo/recomp/blob/HEAD/docs/x.md) | 2 |\n");
}
