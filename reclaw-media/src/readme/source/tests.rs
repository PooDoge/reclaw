use super::*;

fn github() -> ReadmeContext {
    ReadmeContext::github("octo", "recomp", "HEAD").expect("valid")
}

#[test]
fn the_readme_is_at_the_hosts_raw_address() {
    assert_eq!(github().readme_url().as_str(), "https://raw.githubusercontent.com/octo/recomp/HEAD/README.md");
    let gl = ReadmeContext::gitlab("group/sub", "game", "main").expect("valid");
    assert_eq!(gl.readme_url().as_str(), "https://gitlab.com/group/sub/game/-/raw/main/README.md");
}

#[test]
fn names_that_would_change_the_address_are_refused() {
    for (owner, name) in [("", "r"), ("o", ""), ("o/x", "r"), ("..", "r"), ("o", "r/../x"), ("o", "r?x=1"), (".hidden", "r"), ("o w", "r")]
    {
        assert_eq!(ReadmeContext::github(owner, name, "HEAD"), None, "{owner:?}/{name:?}");
    }
    assert_eq!(ReadmeContext::github("o", "r", "a/../b"), None);
    assert_eq!(ReadmeContext::github("o", "r", ""), None);
    assert!(ReadmeContext::github("o", "r", "release/1.0").is_some(), "a branch may contain a slash");
}

#[test]
fn relative_images_resolve_against_the_raw_folder() {
    let c = github();
    assert_eq!(c.image("docs/shot.png").as_deref(), Some("https://raw.githubusercontent.com/octo/recomp/HEAD/docs/shot.png"));
    assert_eq!(c.image("./logo.svg").as_deref(), Some("https://raw.githubusercontent.com/octo/recomp/HEAD/logo.svg"));
    assert_eq!(
        c.image("/assets/a.png").as_deref(),
        Some("https://raw.githubusercontent.com/octo/recomp/HEAD/assets/a.png"),
        "a leading slash is the repository's root"
    );
}

#[test]
fn a_relative_path_cannot_climb_out_of_the_repository() {
    let c = github();
    assert_eq!(c.image("../../other/repo/HEAD/a.png"), None);
    assert_eq!(c.image("../x.png"), None);
    assert_eq!(
        c.image("docs/../a.png").as_deref(),
        Some("https://raw.githubusercontent.com/octo/recomp/HEAD/a.png"),
        "going up inside it is fine"
    );
}

#[test]
fn absolute_images_are_kept_and_http_is_upgraded() {
    let c = github();
    assert_eq!(c.image("https://img.shields.io/badge/a-b-green").as_deref(), Some("https://img.shields.io/badge/a-b-green"));
    assert_eq!(c.image("http://example.com/a.png").as_deref(), Some("https://example.com/a.png"));
    assert_eq!(c.image("//example.com/a.png").as_deref(), Some("https://example.com/a.png"));
}

#[test]
fn images_that_cannot_be_fetched_have_no_address() {
    let c = github();
    for src in ["data:image/png;base64,AAAA", "javascript:alert(1)", "file:///etc/passwd", "", "   ", "ftp://x/y.png"] {
        assert_eq!(c.image(src), None, "{src:?}");
    }
}

#[test]
fn links_go_to_the_web_page_of_the_file_not_the_raw_one() {
    let c = github();
    assert_eq!(c.link("docs/BUILDING.md"), LinkTarget::Web("https://github.com/octo/recomp/blob/HEAD/docs/BUILDING.md".into()));
    assert_eq!(c.link("https://example.com/x"), LinkTarget::Web("https://example.com/x".into()));
    assert_eq!(c.link("/LICENSE"), LinkTarget::Web("https://github.com/octo/recomp/blob/HEAD/LICENSE".into()));
}

#[test]
fn anchors_stay_in_the_page_and_unsafe_links_go_nowhere() {
    let c = github();
    assert_eq!(c.link("#installation"), LinkTarget::Anchor("installation".into()));
    for href in ["#", "javascript:alert(1)", "mailto:a@b.c", "../../other", ""] {
        assert_eq!(c.link(href), LinkTarget::None, "{href:?}");
    }
}
