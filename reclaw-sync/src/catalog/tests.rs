use reclaw_net::{Net, NetConfig, testing::local_config};

use super::*;

#[test]
fn the_default_index_is_the_community_catalog_on_the_web() {
    assert!(DEFAULT_INDEX_URL.starts_with("https://raw.githubusercontent.com/") && DEFAULT_INDEX_URL.ends_with("/index.json"));
}

#[test]
fn without_a_cache_folder_nothing_is_ever_saved() {
    let net = Net::new(NetConfig { cache_dir: None, ..local_config() }).expect("net");
    assert!(CatalogSync::new(net).with_index_url("http://127.0.0.1:1/index.json").saved().is_none());
}

#[test]
fn the_waits_and_limits_are_the_documented_ones() {
    assert_eq!(DEFAULT_TTL, Duration::from_secs(300));
    assert_eq!(PLATFORM_LIMIT, 16 * 1024 * 1024);
}
