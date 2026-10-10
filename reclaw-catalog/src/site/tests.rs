use serde_json::json;

use super::*;
use crate::{entry::AppEntry, source::RepoSource};

/// An entry as the API answered it to Quiver's tests (fields Reclaw does not read included, to show they are ignored).
const G1R_DELUXE: &str = r#"{"addedAt":1788046368000.25,"aiLevel":"assisted","artwork":"https://example.com/cover.png",
 "basedOn":{"console":"gb"},"consoles":["gb","gbc","gba"],"description":"G1R Deluxe brings Pokemon RBY / GSC / FRLG / RSE to PC.",
 "developer":{"key":"bryanthaboi","name":"bryanthaboi"},
 "games":[{"id":"k9","slug":"pokemon-red","title":"Pokemon Red"},{"id":"k8","slug":"pokemon-gold","title":"Pokemon Gold"}],
 "id":"k17env62k9jdbccmgg4hvabjk58f4dhm","lastReleaseAt":1791309882000,"lastReleaseVersion":"v0.3.56",
 "launcher":{"filesToAdd":["portable.txt"],"folderName":"PokemonRedBlueYellowGoldSilverCrystal-Gen1RecompProject",
   "mods":{"layout":"folderPerMod","path":"mods","sources":[{"provider":"gamebanana","sourceUrl":"https://gamebanana.com/games/25428"}]}},
 "makers":["Nintendo"],"name":"Pokemon Red / Blue / Yellow / Gold / Silver / Crystal",
 "projectName":"G1R Deluxe","projectType":"port","recommended":1,"reportBroken":0,"reportIssues":0,"reviewCount":1,
 "slug":"pokemonredblueyellowgoldsilvercrystal-gen1recompproject","supportedOS":["android","linux","macos","windows"],
 "tags":["recreation","gb","pokemon","nintendo","mod support"],
 "verified":{"pinned":true,"prerelease":false,"releasedAt":1791027079000,"version":"v0.3.51"}}"#;

#[test]
fn an_app_reads_with_what_players_said_and_what_was_verified() {
    let page: Page<SiteApp> = parse_page(&format!(r#"{{"items":[{G1R_DELUXE}],"nextCursor":"c2","isDone":false}}"#)).expect("a page");
    assert_eq!(page.next_cursor.as_deref(), Some("c2"));
    let app = &page.items[0];
    assert_eq!(app.project_name, "G1R Deluxe");
    assert_eq!(app.games.iter().map(|g| g.title.as_str()).collect::<Vec<_>>(), ["Pokemon Red", "Pokemon Gold"]);
    assert_eq!(app.supported_os, ["android", "linux", "macos", "windows"]);
    assert_eq!(app.launcher.folder_name, "PokemonRedBlueYellowGoldSilverCrystal-Gen1RecompProject");
    assert_eq!((app.recommended, app.report_issues, app.report_broken, app.review_count), (1, 0, 0, 1));
    assert_eq!(app.ai_level, AiLevel::Assisted);
    assert_eq!(app.developer.as_ref().map(|d| d.name.as_str()), Some("bryanthaboi"));
    let verified = app.verified.as_ref().expect("verified");
    assert_eq!((verified.version.as_str(), verified.pinned), ("v0.3.51", true));
    assert_eq!(app.added_at, 1_788_046_368_000.25, "a fraction of a millisecond is fine");
}

#[test]
fn a_finished_page_has_no_cursor_and_an_unreadable_item_is_left_out_and_counted() {
    let page: Page<SiteApp> =
        parse_page(r#"{"items":[{"slug":"a"}, 7, {"slug":"b","recommended":"lots"}],"nextCursor":"c3","isDone":true}"#).expect("a page");
    assert_eq!(page.next_cursor, None, "isDone wins over a cursor");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.skipped, 2);
    assert!(parse_page::<SiteApp>(r#"{"error":{"message":"no"}}"#).is_err(), "not a page at all");
}

#[test]
fn nulls_and_odd_numbers_take_their_defaults() {
    let app: SiteApp = serde_json::from_value(json!({
        "slug": "x", "name": null, "tags": null, "recommended": -3, "reportIssues": 2.0, "addedAt": null, "aiLevel": "someday"
    }))
    .expect("lenient");
    assert_eq!((app.name.as_str(), app.tags.len(), app.recommended, app.report_issues, app.added_at), ("", 0, 0, 2, 0.));
    assert_eq!(app.ai_level, AiLevel::None, "an unknown level is no claim at all");
}

#[test]
fn release_states_and_verdicts_that_are_not_known_count_as_the_careful_ones() {
    let releases: Page<HistoryRelease> = parse_page(
        r#"{"items":[
          {"version":"v3","state":"blocked","reasons":["A file was replaced."],"scan":{"verdict":"flagged","engines":"9 of 70 engines"},
           "assets":[{"filename":"g-linux.zip","url":"https://x/g-linux.zip","checksum":"sha256:ab","scan":{"verdict":"clean"}}]},
          {"version":"v2","state":"quarantined","scan":{"verdict":"new-kind"}},
          {"version":"v1","state":"verified","prerelease":true}],"isDone":true}"#,
    )
    .expect("a page");
    let states: Vec<ReleaseState> = releases.items.iter().map(|r| r.state).collect();
    assert_eq!(states, [ReleaseState::Blocked, ReleaseState::Unverified, ReleaseState::Verified]);
    assert_eq!(releases.items[0].scan.as_ref().map(|s| s.verdict), Some(Verdict::Flagged));
    assert_eq!(releases.items[0].assets[0].checksum.as_deref(), Some("sha256:ab"));
    assert_eq!(releases.items[1].scan.as_ref().map(|s| s.verdict), Some(Verdict::Pending));
}

#[test]
fn a_review_and_an_apps_page_read() {
    let reviews: Page<Review> = parse_page(
        r#"{"items":[{"id":"r1","author":"Sam","result":"issues","body":"Audio crackles.","platform":"linux","version":"v1.2","createdAt":1791000000000},
                     {"id":"r2","author":"","result":"meh","body":null}],"isDone":true}"#,
    )
    .expect("a page");
    assert_eq!(reviews.items[0].result, RunResult::Issues);
    assert_eq!(reviews.items[1].result, RunResult::Other);
    assert_eq!(reviews.items[1].body, "");

    let detail: Detail = parse_one(&format!(
        r#"{{"entry":{G1R_DELUXE},"project":{{"name":"G1R","provider":"github","repository":"bryanthaboi/pokemon-gen1-recomp-project",
            "author":"bryanthaboi","aiUse":{{"level":"assisted","source":"readme","evidence":[{{"kind":"readme","detail":"Says Copilot helped."}}]}}}},
            "withdrawn":[{{"version":"v0.3.40","reason":"Crashed on start.","at":1790000000000}}],
            "checking":{{"version":"v0.3.56","checkEndsAt":1791400000000,"needsReview":false,"reasons":["Waiting 48 hours."]}}}}"#
    ))
    .expect("readable")
    .expect("not null");
    assert_eq!(detail.project.author.as_deref(), Some("bryanthaboi"));
    assert_eq!(detail.project.ai_use.as_ref().map(|a| a.evidence.len()), Some(1));
    assert_eq!(detail.withdrawn[0].reason, "Crashed on start.");
    assert_eq!(detail.checking.as_ref().map(|c| c.version.as_str()), Some("v0.3.56"));
    assert_eq!(parse_one::<Detail>("null"), Ok(None));
}

#[test]
fn a_page_with_withdrawn_false_reads_as_the_live_api_answers_it() {
    // Quiver 3.5's test of a real `/apps/{slug}` answer has `"withdrawn":false`, and `game`/`games` beside the entry.
    let detail: Detail = parse_one(&format!(
        r#"{{"entry":{G1R_DELUXE},"game":{{"artwork":"https://x/icon.png","id":"k9"}},"games":[],
            "project":{{"aiUse":{{"level":"assisted","source":"signals"}},"author":"bryanthaboi","name":"G1R Deluxe",
              "projectType":"port","provider":"github","repository":"bryanthaboi/gen1recomp","slug":"github-bryanthaboi-gen1recomp"}},
            "withdrawn":false}}"#
    ))
    .expect("readable")
    .expect("not null");
    assert!(detail.withdrawn.is_empty());
    assert_eq!(detail.project.repository.as_deref(), Some("bryanthaboi/gen1recomp"));
    assert_eq!(detail.entry.consoles, ["gb", "gbc", "gba"]);
    assert_eq!(detail.entry.launcher.files_to_add, ["portable.txt"]);
}

#[test]
fn addresses_escape_what_they_carry() {
    assert_eq!(path::apps(None), "/apps?limit=100");
    assert_eq!(path::release_status(Some("a b/c")), "/release-status?limit=100&cursor=a%20b%2Fc");
    assert_eq!(path::reviews("zelda-64", REVIEWS_SHOWN), "/apps/zelda-64/reviews?limit=10");
    assert_eq!(path::detail("../x"), "/apps/..%2Fx");
    assert_eq!(review_page_url("sm64"), "https://quiverlauncher.com/apps/sm64?tab=how-it-runs");
}

fn status(id: &str, slug: &str, provider: &str, repository: &str) -> ReleaseStatus {
    ReleaseStatus { id: id.into(), slug: slug.into(), provider: provider.into(), repository: Some(repository.into()), ..Default::default() }
}

fn app(slug: &str, folder: &str, filter: Option<&str>) -> SiteApp {
    SiteApp {
        slug: slug.into(),
        launcher: Launcher { folder_name: folder.into(), release_asset_filter: filter.map(str::to_string), ..Default::default() },
        ..Default::default()
    }
}

fn entry(repository: &str, folder: &str) -> AppEntry {
    AppEntry { name: "Game".into(), repository: repository.into(), folder_name: folder.into(), ..Default::default() }
}

#[test]
fn an_app_links_by_its_repository_ignoring_case_and_only_on_its_provider() {
    let links = Links::new(vec![status("1", "sm64", "github", "Owner/SM64"), status("2", "lab", "gitlab", "group/sub/game")], vec![]);
    assert_eq!(link(&entry("owner/sm64 ", "Anything"), &links).as_deref(), Some("sm64"));
    assert_eq!(link(&entry("group/sub/game", "x"), &links), None, "the same path on GitHub is another project");
    let lab = AppEntry { source: RepoSource::Gitlab, ..entry("group/sub/game", "x") };
    assert_eq!(link(&lab, &links).as_deref(), Some("lab"));
    assert_eq!(link(&AppEntry { repository: String::new(), ..entry("", "sm64") }, &links), None, "a manual app has no entry");
}

#[test]
fn the_entry_id_wins_then_the_lists_catalog_id() {
    let links = Links::new(
        vec![status("k1", "renamed", "github", "new-owner/game"), status("k2", "other", "github", "o/r")],
        vec![SiteApp { catalog_id: Some("qcat_9".into()), ..app("other", "F", None) }],
    );
    let by_id = AppEntry { catalog_entry_id: Some("k1".into()), ..entry("o/r", "F") };
    assert_eq!(link(&by_id, &links).as_deref(), Some("renamed"), "the id survives a move and beats the repository");
    let by_catalog = AppEntry { catalog_id: Some("QCAT_9".into()), ..entry("elsewhere/x", "Y") };
    assert_eq!(link(&by_catalog, &links).as_deref(), Some("other"));
}

#[test]
fn one_repository_with_several_games_is_told_apart_by_filter_then_folder_and_never_guessed() {
    let repo = "mstan/FireRedLeafGreenRecomp";
    let statuses = vec![status("1", "red", "github", repo), status("2", "green", "github", repo)];
    let links = Links::new(
        statuses.clone(),
        vec![app("red", "PokemonFireRed", Some("FireRed")), app("green", "PokemonLeafGreen", Some("LeafGreen"))],
    );
    let green = AppEntry { release_asset_filter: Some("leafgreen".into()), ..entry(repo, "Elsewhere") };
    assert_eq!(link(&green, &links).as_deref(), Some("green"), "the filter, ignoring case");
    assert_eq!(link(&entry(repo, "pokemonfirered"), &links).as_deref(), Some("red"), "no filter: the folder");
    assert_eq!(link(&entry(repo, "Neither"), &links), None, "two candidates and nothing to choose by");
    assert_eq!(link(&entry(repo, "PokemonFireRed"), &Links::new(statuses, vec![])), None, "no listing: nothing to choose by");
}

#[test]
fn a_repository_that_moved_is_found_by_its_folder_when_only_one_entry_has_it() {
    let links = Links::new(
        vec![status("1", "a", "github", "new/a"), status("2", "b", "github", "new/b")],
        vec![app("a", "GameA", None), app("b", "", None)],
    );
    assert_eq!(link(&entry("old/a", "gamea"), &links).as_deref(), Some("a"));
    assert_eq!(link(&entry("old/b", "b"), &links).as_deref(), Some("b"), "no folder from the site: its slug");
    assert_eq!(link(&entry("old/c", "GameC"), &links), None);
}

#[test]
fn a_slug_of_dots_never_becomes_a_path_segment() {
    assert_eq!(path::release_history(".."), "/apps/_/release-history?limit=100");
    assert_eq!(path::download_problem("."), "/apps/_/download-problem");
}
