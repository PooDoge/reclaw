use reclaw_catalog::site::{
    AiLevel, Checking, Developer, Launcher, ReleaseState, ReleaseStatus, RunResult, Scan, SiteProject, Verdict, Verified,
};
use reclaw_sync::CatalogApp;

use super::*;

const NOW: f64 = 1_791_000_000_000.;
const HOUR: f64 = 3_600_000.;

#[test]
fn the_rating_is_what_most_said_and_a_tie_goes_to_the_careful_answer() {
    let r = Rating::new(4, 0, 0);
    assert_eq!((r.line(), r.tone, r.players), ("Runs well · 4 players".to_string(), Tone::Positive, 4));
    assert_eq!(Rating::new(1, 0, 0).line(), "Runs well · 1 player");
    assert_eq!(Rating::new(2, 1, 0).line(), "Runs well · 2 run well, 1 with issues");
    assert_eq!(Rating::new(1, 1, 0).label, "Has issues", "a tie between runs and issues");
    assert_eq!(Rating::new(1, 1, 1).line(), "Doesn't run · 1 runs well, 1 with issues, 1 doesn't run");
    assert_eq!(Rating::new(0, 0, 2).line(), "Doesn't run · 2 players");
    let none = Rating::new(0, 0, 0);
    assert_eq!((none.line(), none.tone), ("Not rated yet".to_string(), Tone::Muted));
    assert_eq!(share_title(0), "Nobody has shared how it runs yet");
}

#[test]
fn a_release_age_reads_like_the_websites() {
    assert_eq!(release_age(None, NOW), "No releases");
    assert_eq!(release_age(Some(NOW - 10. * 60_000.), NOW), "Updated just now");
    assert_eq!(release_age(Some(NOW - 5. * HOUR), NOW), "Updated 5 h ago");
    assert_eq!(release_age(Some(NOW - 3. * 24. * HOUR), NOW), "Updated 3 d ago");
    assert_eq!(release_age(Some(NOW - 65. * 24. * HOUR), NOW), "Updated 2 mo ago");
    assert_eq!(release_age(Some(NOW - 800. * 24. * HOUR), NOW), "No release in 2 yr+");
}

#[test]
fn days_are_civil_dates() {
    assert_eq!(day(0.), "1 Jan 1970");
    assert_eq!(day(951_782_400_000.), "29 Feb 2000");
    assert_eq!(day(1_791_000_000_000.), "3 Oct 2026");
}

fn app() -> SiteApp {
    SiteApp {
        slug: "sm64".into(),
        supported_os: vec!["linux".into(), "windows".into(), "beos".into()],
        developer: Some(Developer { name: "Ghostship".into() }),
        last_release_at: Some(NOW - 3. * 24. * HOUR),
        last_release_version: Some("v1.2".into()),
        verified: Some(Verified { version: "v1.1".into(), ..Default::default() }),
        ai_level: AiLevel::Assisted,
        ..Default::default()
    }
}

#[test]
fn the_facts_name_the_maker_platforms_releases_and_ai_use() {
    let facts = facts(&app(), None, NOW);
    let value = |term: &str| facts.iter().find(|(t, _)| *t == term).map(|(_, v)| v.clone());
    assert_eq!(value("Made by").as_deref(), Some("Ghostship"));
    assert_eq!(value("Platforms").as_deref(), Some("Windows, Linux"), "the website's order; an unknown one left out");
    assert_eq!(value("Latest release").as_deref(), Some("v1.2 · Updated 3 d ago"));
    assert_eq!(value("Verified").as_deref(), Some("v1.1"));
    assert_eq!(value("AI use").as_deref(), Some("AI-assisted"));

    let project = SiteProject { author: Some("The Author".into()), ..Default::default() };
    let bare = SiteApp { developer: None, supported_os: vec![], last_release_at: None, verified: None, ..app() };
    let facts = super::facts(&bare, Some(&project), NOW);
    assert_eq!(facts[0], ("Made by", "The Author".to_string()), "the project's author first");
    assert!(facts.contains(&("Platforms", "Not confirmed yet".to_string())));
    assert!(facts.contains(&("Latest release", "No releases found".to_string())));
    assert!(facts.contains(&("Verified", "None yet".to_string())));
    assert_eq!(ai_chip(AiLevel::None), None);
}

#[test]
fn a_review_line_says_who_when_where_and_how_it_ran() {
    let review = Review {
        author: "  ".into(),
        result: RunResult::Issues,
        body: " Audio crackles. ".into(),
        platform: Some("linux".into()),
        version: Some("v1.2".into()),
        created_at: NOW,
        ..Default::default()
    };
    let line = ReviewLine::new(&review);
    assert_eq!(line.author, "A player");
    assert_eq!(line.meta, "3 Oct 2026 · Linux · tested on v1.2");
    assert_eq!((line.result, line.tone, line.body.as_str()), ("Runs with issues", Tone::Caution, "Audio crackles."));
    let bare = ReviewLine::new(&Review { author: "Sam".into(), result: RunResult::Runs, ..Default::default() });
    assert_eq!((bare.meta.as_str(), bare.result), ("", "Runs well"));
}

#[test]
fn a_release_line_gives_its_state_why_and_what_virustotal_said_when_it_matters() {
    let blocked = HistoryRelease {
        version: "v3".into(),
        state: ReleaseState::Blocked,
        reasons: vec!["A file was replaced after release.".into(), " ".into()],
        scan: Some(Scan { verdict: Verdict::Flagged, engines: Some("9 of 70 engines".into()), url: None }),
        ..Default::default()
    };
    let line = ReleaseLine::new(&blocked, NOW);
    assert_eq!((line.state, line.tone), ("Blocked", Tone::Negative));
    assert_eq!(
        line.reasons,
        ["A file was replaced after release.", "VirusTotal: 9 of 70 engines flag one of its files. Your antivirus may block or remove it."]
    );
    let waiting = HistoryRelease {
        version: "v4".into(),
        state: ReleaseState::Unverified,
        reasons: vec!["New releases wait 48 hours.".into()],
        check_ends_at: Some(NOW + 4.5 * HOUR),
        scan: Some(Scan { verdict: Verdict::Clean, ..Default::default() }),
        prerelease: true,
        ..Default::default()
    };
    let line = ReleaseLine::new(&waiting, NOW);
    assert_eq!(line.reasons, ["New releases wait 48 hours.", "It should be verified in about 5 hours."], "a clean scan says nothing");
    assert!(line.prerelease);
    let warned = Scan { verdict: Verdict::Warning, engines: None, url: None };
    assert_eq!(scan_line(Some(&warned)).as_deref(), Some("VirusTotal: an engine flag one of its files (often a false alarm)."));
}

#[test]
fn a_release_being_checked_says_why_and_how_long() {
    let checking =
        Checking { version: "v5".into(), check_ends_at: Some(NOW + HOUR / 2.), reasons: vec!["New developer".into()], needs_review: false };
    assert_eq!(
        checking_line(&checking, NOW).as_deref(),
        Some("v5 is being checked. New developer. It should be verified in about 1 hour.")
    );
    let held = Checking { needs_review: true, ..checking };
    assert_eq!(checking_line(&held, NOW).as_deref(), Some("v5 is being checked. New developer. A maintainer has to look at it first."));
}

fn catalog_app(repository: &str, folder: &str) -> CatalogApp {
    CatalogApp {
        entry: AppEntry { name: folder.into(), repository: repository.into(), folder_name: folder.into(), ..Default::default() },
        lists: vec![],
        release: None,
    }
}

#[test]
fn every_game_the_screens_know_is_linked_by_its_id_and_the_librarys_copy_wins() {
    let catalog = vec![catalog_app("o/sm64", "SM64"), catalog_app("o/unlisted", "Other")];
    let status = |id: &str, slug: &str, repo: &str| ReleaseStatus {
        id: id.into(),
        slug: slug.into(),
        provider: "github".into(),
        repository: Some(repo.into()),
        ..Default::default()
    };
    let site_app = |slug: &str, recommended: u32| SiteApp {
        slug: slug.into(),
        recommended,
        launcher: Launcher { folder_name: slug.into(), release_asset_filter: None },
        ..Default::default()
    };
    let links = Links::new(
        vec![status("k1", "sm64", "o/sm64"), status("k2", "sm64-moved", "elsewhere/sm64")],
        vec![site_app("sm64", 3), site_app("sm64-moved", 9)],
    );
    let linked = link_all(&catalog, &[], &links);
    let ids = IdMap::for_keys(catalog.iter().map(|a| key_of(&a.entry)));
    let sm64 = ids.get(&key_of(&catalog[0].entry)).expect("an id");
    assert_eq!(linked.len(), 1, "the unlisted app has no entry");
    assert_eq!(linked[&sm64].recommended, 3);

    // The same game in the library, with the entry id Quiver wrote when it was linked to the moved entry.
    let library = vec![AppEntry { catalog_entry_id: Some("k2".into()), ..catalog[0].entry.clone() }];
    assert_eq!(link_all(&catalog, &library, &links)[&sm64].recommended, 9);
}
