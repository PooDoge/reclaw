use reclaw_catalog::{CommunityIndex, parse_list};

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
    CatalogSnapshot { index, index_origin: Origin::Network, lists, platform: None, problems: vec![], fetched_at: 0 }
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
