use reclaw_catalog::site::{Asset as SiteAsset, Scan};

use super::*;

fn file(name: &str, sum: Option<&str>) -> SiteAsset {
    SiteAsset {
        filename: name.into(),
        url: Some(format!("https://github.com/o/r/releases/download/x/{name}")),
        checksum: sum.map(|s| format!("sha256:{s}")),
        size: Some(10.),
        scan: None,
    }
}

fn release(version: &str, state: ReleaseState, prerelease: bool) -> HistoryRelease {
    HistoryRelease {
        version: version.into(),
        state,
        prerelease,
        assets: vec![file("g-linux.zip", Some(&"AB".repeat(32)))],
        ..Default::default()
    }
}

#[test]
fn a_site_release_keeps_only_https_files_and_lowercases_their_checksums() {
    let mut r = release("v1", ReleaseState::Verified, false);
    r.assets.push(SiteAsset { url: Some("http://x.test/g.zip".into()), ..file("plain.zip", None) });
    r.assets.push(file("odd.zip", Some("not-hex")));
    let out = to_release(&r);
    assert_eq!(out.assets.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["g-linux.zip", "odd.zip"]);
    assert_eq!(out.assets[0].sha256.as_deref(), Some("ab".repeat(32).as_str()));
    assert_eq!((out.assets[0].size, out.assets[1].sha256.as_deref()), (Some(10), None));
    r.rolling = true;
    assert_eq!(to_release(&r).assets[0].sha256, None, "a rolling build may have changed since the site read it");
}

#[test]
fn the_target_is_the_pin_then_the_verified_release_then_the_newest_not_blocked() {
    let history = vec![
        release("v4-beta", ReleaseState::Unverified, true),
        release("v3", ReleaseState::Blocked, false),
        release("v2", ReleaseState::Unverified, false),
        release("v1", ReleaseState::Verified, false),
    ];
    let pick = |pin, verified, pre| target(&history, pin, verified, pre).map(|r| r.version.as_str());
    assert_eq!(pick(Some("3"), None, false), Some("v3"), "a pin wins, even a blocked one (it is confirmed twice)");
    assert_eq!(pick(Some("v0"), None, false), None, "a pin the site does not list is for the host");
    assert_eq!(pick(None, Some("v1"), false), Some("v1"));
    assert_eq!(pick(None, None, false), Some("v1"), "the newest verified, when the feed named none");
    let unverified = &history[..3];
    assert_eq!(target(unverified, None, None, false).map(|r| r.version.as_str()), Some("v2"), "stable before pre-release, never blocked");
    assert_eq!(target(unverified, None, None, true).map(|r| r.version.as_str()), Some("v4-beta"));
    let nightly = [release("n2", ReleaseState::Unverified, false), release("n1", ReleaseState::Verified, true)];
    assert_eq!(target(&nightly, None, Some("n1"), false).map(|r| r.version.as_str()), Some("n1"), "a verified pre-release is installed");
    let mut gone = release("v5", ReleaseState::Verified, false);
    gone.assets.clear();
    let feed_without_files = [gone, release("v1", ReleaseState::Verified, false)];
    assert_eq!(target(&feed_without_files, None, Some("v5"), false).map(|r| r.version.as_str()), Some("v1"), "else the newest verified");
}

#[test]
fn a_file_the_site_did_not_check_is_unverified_and_a_flagged_one_asks() {
    let mut r = release("v1", ReleaseState::Verified, false);
    assert_eq!(check(&r, "G-Linux.zip"), Check::Verified);
    let other = check(&r, "g-windows.zip");
    assert!(matches!(&other, Check::Unverified(why) if why[0].contains("not g-windows.zip")), "{other:?}");
    r.assets[0].scan = Some(Scan { verdict: Verdict::Flagged, engines: Some("5 of 70 engines".into()), url: None });
    assert_eq!(check(&r, "g-linux.zip"), Check::Flagged { engines: Some("5 of 70 engines".into()) });
    assert_eq!(check(&r, "g-linux.zip").confirmations(), 1);
    let blocked = release("v2", ReleaseState::Blocked, false);
    assert_eq!(check(&blocked, "g-linux.zip").confirmations(), 2);
    assert_eq!(check_unlisted("1", Some("v1"), true), Check::Verified);
    assert_eq!(check_unlisted("v9", Some("v1"), false), Check::Unverified(vec![UNREACHABLE.into()]));
}

#[test]
fn a_release_is_confirmed_by_pressing_again_in_time_and_a_blocked_one_by_pressing_twice() {
    let mut c = Confirmations::default();
    let t = Instant::now();
    assert!(c.press(1, "v2", 0, t), "verified: no confirmation");
    assert!(!c.press(1, "v2", 1, t));
    assert_eq!(c.remaining(1, 1), 1);
    assert!(c.press(1, "v2", 1, t + Duration::from_secs(5)));
    assert!(!c.press(1, "v2", 1, t), "cleared once confirmed");
    assert!(!c.press(1, "v2", 1, t + WINDOW + Duration::from_secs(1)), "too late: starts over");
    assert!(!c.press(1, "v3", 1, t + WINDOW + Duration::from_secs(2)), "another release starts over");

    assert!(!c.press(2, "v3", 2, t));
    assert!(!c.press(2, "v3", 2, t));
    assert_eq!(c.remaining(2, 2), 1);
    assert!(c.press(2, "v3", 2, t));
}

#[test]
fn an_offered_release_is_taken_once_and_only_in_time() {
    let mut c = Confirmations::default();
    let t = Instant::now();
    c.offer(1, "v1", t);
    assert_eq!(c.take_offer(1, t), Some("v1".into()));
    assert_eq!(c.take_offer(1, t), None);
    c.offer(1, "v1", t);
    assert_eq!(c.take_offer(1, t + WINDOW + Duration::from_secs(1)), None);
    let history = [release("v2", ReleaseState::Verified, false), release("v1", ReleaseState::Verified, false)];
    assert_eq!(fallback(&history, "2").map(|r| r.version.as_str()), Some("v1"));
}
