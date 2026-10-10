use reclaw_games::listing::Ratings;

use super::*;
use crate::{
    fixtures::sample_projects,
    settings::{SettingValue, SettingsValues},
};

/// The sample projects with site facts: 1 old and loved, 2 new and AI-assisted, 3 a Windows-only tool, the rest only in the lists.
fn projects() -> Vec<ProjectInfo> {
    let mut projects = sample_projects();
    let listing = |added: u64, updated: u64, runs: u32, broken: u32, kind: &str, os: &[&str], ai: AiUse| Listing {
        added_at: Some(added),
        updated_at: Some(updated),
        ratings: Ratings { runs, issues: 0, broken },
        project_type: kind.into(),
        runs_on: os.iter().map(|o| (*o).to_string()).collect(),
        ai,
        ..Default::default()
    };
    projects[0].listing = Some(listing(100, 900, 9, 0, "port", &[], AiUse::None));
    projects[1].listing = Some(listing(300, 500, 1, 2, "port", &["linux", "windows", "macos"], AiUse::Assisted));
    projects[2].listing = Some(listing(200, 700, 0, 0, "tool", &["windows-only-os"], AiUse::Generated));
    projects
}

fn titles(browse: &Browse) -> Vec<String> {
    browse.entries.iter().map(|g| g.title.to_string()).collect()
}

fn all() -> CatalogView {
    CatalogView { this_system_only: false, hide_library: false, ..Default::default() }
}

#[test]
fn the_sorts_order_by_the_sites_facts_and_the_lists_own_apps_come_last() {
    let projects = projects();
    let by = |sort| titles(&browse(&[], &projects, None, "", &CatalogView { sort, ..all() }));
    let added = by(CatalogSort::Added);
    assert_eq!(added.len(), projects.len(), "nothing filtered");
    assert_eq!(added, projects.iter().map(|p| p.title.clone()).collect::<Vec<_>>(), "Added keeps the host's order");
    assert_eq!(by(CatalogSort::Updated)[..3], ["Starfall 64", "Kart Ruins", "Skyward Quest"]);
    assert_eq!(
        by(CatalogSort::TopRated)[..3],
        ["Starfall 64", "Kart Ruins", "Skyward Quest"],
        "loved, then unrated on the site (a half), then mostly broken"
    );
    assert_eq!(by(CatalogSort::Name)[0], "Dino Rush");
}

#[test]
fn the_filters_judge_what_the_site_said_and_keep_what_they_cannot_judge() {
    let projects = projects();
    let count = |view: CatalogView| browse(&[], &projects, None, "", &view).entries.len();
    let total = projects.len();
    assert_eq!(count(CatalogView { this_system_only: true, ..all() }), total - 1, "the tool runs elsewhere only");
    assert_eq!(count(CatalogView { kind: Some("port"), ..all() }), 2, "a kind needs the site to say so");
    assert_eq!(count(CatalogView { ai: AiFilter::NoGenerated, ..all() }), total - 1);
    assert_eq!(count(CatalogView { ai: AiFilter::NoAi, ..all() }), total - 2);
}

#[test]
fn apps_in_the_library_are_hidden_and_counted() {
    let projects = projects();
    let mut games = vec![GameEntry::from_project(&projects[0])];
    games[0].in_library = true;
    let shown = browse(&games, &projects, None, "", &CatalogView { hide_library: true, ..all() });
    assert_eq!((shown.entries.len(), shown.hidden_in_library), (projects.len() - 1, 1));
    let everything = browse(&games, &projects, None, "", &all());
    assert!(everything.entries[0].in_library, "shown as the library's copy when not hidden");
}

#[test]
fn the_settings_map_to_a_view_and_the_defaults_are_quivers() {
    let default = CatalogView::from_settings(&SettingsValues::default());
    assert_eq!(default, CatalogView::default());
    assert!(default.this_system_only && default.hide_library && default.kind.is_none());
    let mut values = SettingsValues::default();
    values.set(SettingsTarget::Global, KEY_CATALOG_SORT, SettingValue::Choice(2));
    values.set(SettingsTarget::Global, KEY_CATALOG_KIND, SettingValue::Choice(3));
    values.set(SettingsTarget::Global, KEY_CATALOG_AI, SettingValue::Choice(9));
    let view = CatalogView::from_settings(&values);
    assert_eq!((view.sort, view.kind, view.ai), (CatalogSort::TopRated, Some("emulator"), AiFilter::All), "out of range is the default");
    assert_eq!(SORT_OPTIONS.len(), CatalogSort::ALL.len());
}

#[test]
fn a_card_says_how_it_runs_and_whether_it_is_new() {
    let notes = card_notes(&projects(), 350);
    let starfall = &notes[&projects()[0].id];
    assert_eq!((starfall.rating.as_str(), starfall.tone, starfall.new), ("Runs well · 9 players", Tone::Positive, true));
    assert_eq!(notes[&projects()[2].id].rating, "Not rated yet");
    assert!(!notes.contains_key(&projects()[4].id), "a list-only project has nothing to say");
    assert!(!card_notes(&projects(), 100 + 31 * 24 * 3600)[&projects()[0].id].new);
}
