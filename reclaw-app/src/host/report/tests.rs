use super::*;

fn facts() -> Facts {
    Facts {
        unix_time: 1_790_000_000,
        system: vec![("Reclaw".into(), "0.1.0 (commit abc, debug build)".into())],
        folders: vec![("logs".into(), "/home/u/.local/state/Reclaw/logs".into())],
        environment: vec![("HTTPS_PROXY".into(), "http://user:hunter2@proxy.example:3128".into())],
        tokens: vec![("GitHub".into(), "Saved; sent; 4,987 of 5,000 left".into())],
        services: vec![("GitHub API".into(), "https://api.github.com/rate_limit: ok 200".into())],
        catalog: vec!["232 apps in the catalog".into()],
        log: vec![
            "2026-10-05T10:00:00Z  INFO reclaw_net: fetched".into(),
            "2026-10-05T10:00:01Z  WARN reclaw_net: with ghp_0123456789abcdefghijklmnop in it".into(),
        ],
    }
}

#[test]
fn the_report_has_every_section_in_order() {
    let text = render(&facts());
    let at = |s: &str| text.find(s).unwrap_or_else(|| panic!("{s} missing in:\n{text}"));
    let order =
        ["System", "Folders", "Environment", "Access tokens", "Services, asked from this machine", "Catalog", "End of the log (2 lines)"];
    let positions: Vec<usize> = order.iter().map(|s| at(s)).collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
    assert!(text.contains("Reclaw: 0.1.0 (commit abc, debug build)") && text.contains("GitHub: Saved; sent; 4,987 of 5,000 left"));
    assert!(text.contains("232 apps in the catalog") && text.contains("fetched"));
}

#[test]
fn credentials_are_removed_from_the_whole_report_whatever_put_them_there() {
    let text = render(&facts());
    for leaked in ["hunter2", "ghp_0123456789"] {
        assert!(!text.contains(leaked), "{leaked} leaked:\n{text}");
    }
    assert!(text.contains("proxy.example:3128"), "the host of the proxy is what a reader needs");
    assert!(text.contains("Contains no tokens"));
}
