use reclaw_catalog::{CommunityIndex, parse_list};

use reclaw_catalog::{
    AppEntry,
    site::{Links, SiteApp},
};

use super::*;

fn app(repo: &str, folder: &str) -> String {
    format!(r#"{{"name": "{folder}", "repository": "{repo}", "folderName": "{folder}"}}"#)
}

fn list_snapshot(id: &str, name: &str, apps: &[String]) -> ListSnapshot {
    let text = format!(r#"{{"name": "{name}", "version": "1", "apps": [{}]}}"#, apps.join(","));
    ListSnapshot {
        source: IndexSource { id: id.into(), name: String::new(), description: String::new(), url: format!("https://e.test/{id}.json") },
        list: parse_list(&text).expect("a list"),
        origin: Origin::Network,
        fetched_at: 0,
    }
}

fn snapshot(lists: Vec<ListSnapshot>) -> CatalogSnapshot {
    let index =
        CommunityIndex::parse(r#"{"version": 2, "lists": [{"id": "a", "remoteLocation": "https://e.test/a.json"}]}"#).expect("index");
    CatalogSnapshot { index, index_origin: Origin::Network, lists, platform: None, site: None, problems: vec![], fetched_at: 0 }
}

#[test]
fn an_app_in_two_lists_appears_once_with_both_lists_noted() {
    let snap = snapshot(vec![
        list_snapshot("a", "Nintendo", &[app("o/one", "One"), app("o/two", "Two")]),
        list_snapshot("b", "Other", &[app("O/TWO", "two"), app("o/three", "Three")]),
    ]);
    let apps = snap.apps();
    let names: Vec<_> = apps.iter().map(|a| a.entry.name.as_str()).collect();
    assert_eq!(names, ["One", "Two", "Three"], "first appearance wins and order is kept");
    assert_eq!(apps[1].lists, ["Nintendo", "Other"]);
    assert_eq!(apps[0].lists, ["Nintendo"]);
}

#[test]
fn a_list_without_a_name_is_called_what_the_index_calls_it() {
    let mut list = list_snapshot("a", "", &[app("o/one", "One")]);
    list.list.name = None;
    list.source.name = "From the index".into();
    assert_eq!(snapshot(vec![list]).apps()[0].lists, ["From the index"]);
}

#[test]
fn stale_parts_are_noticed_wherever_they_are() {
    let mut snap = snapshot(vec![list_snapshot("a", "N", &[])]);
    assert!(!snap.has_stale_parts());
    snap.lists[0].origin = Origin::Stale;
    assert!(snap.has_stale_parts());
    snap.lists[0].origin = Origin::Saved;
    snap.index_origin = Origin::Stale;
    assert!(snap.has_stale_parts());
}

#[test]
fn the_origin_follows_where_the_network_layer_got_the_bytes() {
    assert_eq!(Origin::from(Source::Network), Origin::Network);
    assert_eq!(Origin::from(Source::CacheFresh), Origin::Saved);
    assert_eq!(Origin::from(Source::CacheRevalidated), Origin::Revalidated);
    assert_eq!(Origin::from(Source::CacheStale), Origin::Stale);
}

fn site_app(slug: &str, name: &str, folder: &str, added_at: f64) -> SiteApp {
    SiteApp {
        id: format!("id-{slug}"),
        slug: slug.into(),
        name: name.into(),
        added_at,
        launcher: reclaw_catalog::site::Launcher { folder_name: folder.into(), ..Default::default() },
        ..Default::default()
    }
}

fn feed(slug: &str, repo: &str) -> reclaw_catalog::site::ReleaseStatus {
    reclaw_catalog::site::ReleaseStatus {
        id: format!("id-{slug}"),
        slug: slug.into(),
        provider: "github".into(),
        repository: Some(repo.into()),
        ..Default::default()
    }
}

fn with_site(mut snap: CatalogSnapshot, apps: Vec<SiteApp>, status: Vec<reclaw_catalog::site::ReleaseStatus>) -> CatalogSnapshot {
    snap.site = Some(SiteSnapshot { links: Links::new(status, apps), origin: Origin::Network, fetched_at: 0 });
    snap
}

#[test]
fn the_sites_apps_come_first_newest_first_and_the_lists_add_only_what_it_lacks() {
    let mut list = list_snapshot("a", "Nintendo", &[app("o/one", "One"), app("o/old", "Old")]);
    list.list.apps[0].extension = Some(reclaw_catalog::Extension { summary: Some("From the list".into()), ..Default::default() });
    let snap = with_site(
        snapshot(vec![list]),
        vec![site_app("one", "One (site)", "One", 1.0), site_app("new", "Brand New", "New", 2.0), site_app("lost", "No feed", "L", 3.0)],
        vec![feed("one", "o/one"), feed("new", "o/new")],
    );
    let apps = snap.apps();
    let names: Vec<_> = apps.iter().map(|a| a.entry.name.as_str()).collect();
    assert_eq!(names, ["Brand New", "One (site)", "Old"], "newest first; an app missing from the feed is left out; the list's own after");
    assert_eq!(apps[1].lists, [SITE_LIST, "Nintendo"]);
    assert_eq!(apps[1].entry.extension.as_ref().and_then(|e| e.summary.as_deref()), Some("From the list"), "the reclaw block survives");
    assert!(apps[1].site.is_some() && apps[2].site.is_none());
    assert_eq!(apps[0].entry.catalog_entry_id.as_deref(), Some("id-new"));
}

#[test]
fn a_site_app_the_library_holds_takes_the_librarys_folder() {
    let snap = with_site(snapshot(vec![]), vec![site_app("one", "One", "SiteFolder", 1.0)], vec![feed("one", "o/one")]);
    let library = AppEntry {
        name: "One".into(),
        repository: "o/one".into(),
        folder_name: "MyFolder".into(),
        catalog_entry_id: Some("id-one".into()),
        ..Default::default()
    };
    let apps = snap.apps_for(std::slice::from_ref(&library));
    assert!(apps[0].entry.same_instance(&library), "one game, not a tile and a look-alike card");
    assert_eq!(snap.apps()[0].entry.folder_name, "SiteFolder");
}
