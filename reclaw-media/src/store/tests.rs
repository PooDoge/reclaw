use super::*;

fn store() -> (tempfile::TempDir, DiskStore) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = DiskStore::new(dir.path().join("images"));
    (dir, store)
}

fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

const PNG: Kind = Kind::Image(ImageKind::Png);

#[test]
fn what_is_put_comes_back_with_its_kind_and_time() {
    let (_dir, store) = store();
    let put = store.put("https://example.com/a.png", PNG, b"pixels", at(1_000)).expect("put");
    let got = store.get("https://example.com/a.png").expect("hit");
    assert_eq!(got, put);
    assert_eq!((got.kind, got.fetched), (PNG, at(1_000)));
    assert_eq!(fs::read(&got.path).expect("data"), b"pixels");
    assert_eq!(got.path.extension().and_then(|e| e.to_str()), Some("png"));
}

#[test]
fn an_address_never_stored_is_a_miss() {
    let (_dir, store) = store();
    assert_eq!(store.get("https://example.com/none.png"), None);
}

#[test]
fn putting_again_replaces_and_a_changed_kind_leaves_no_old_file() {
    let (_dir, store) = store();
    let url = "https://example.com/logo";
    let first = store.put(url, PNG, b"old", at(1)).expect("put");
    let second = store.put(url, Kind::Image(ImageKind::Svg), b"<svg/>", at(2)).expect("put");
    assert!(!first.path.exists(), "the old PNG is gone");
    assert_eq!(store.get(url), Some(second));
}

#[test]
fn a_name_clash_between_two_addresses_reads_as_a_miss_not_the_wrong_file() {
    let (_dir, store) = store();
    store.put("https://example.com/a.png", PNG, b"a", at(1)).expect("put");
    // Pretend another address hashed to the same name by rewriting the description.
    let stem = store.stem("https://example.com/a.png");
    fs::write(DiskStore::meta_path(&stem), "https://example.com/other.png\n1\npng\n").expect("rewrite");
    assert_eq!(store.get("https://example.com/a.png"), None);
}

#[test]
fn data_without_its_description_is_a_miss() {
    let (_dir, store) = store();
    let entry = store.put("https://example.com/a.png", PNG, b"a", at(1)).expect("put");
    fs::remove_file(DiskStore::meta_path(&store.stem("https://example.com/a.png"))).expect("remove");
    assert!(entry.path.exists());
    assert_eq!(store.get("https://example.com/a.png"), None);
}

#[test]
fn a_garbled_description_is_a_miss() {
    let (_dir, store) = store();
    store.put("https://example.com/a.png", PNG, b"a", at(1)).expect("put");
    let meta = DiskStore::meta_path(&store.stem("https://example.com/a.png"));
    for junk in ["", "https://example.com/a.png\n", "https://example.com/a.png\nsoon\npng\n", "https://example.com/a.png\n1\nexe\n"] {
        fs::write(&meta, junk).expect("rewrite");
        assert_eq!(store.get("https://example.com/a.png"), None, "{junk:?}");
    }
}

#[test]
fn no_partial_file_is_left_after_a_write() {
    let (_dir, store) = store();
    let entry = store.put("https://example.com/a.png", PNG, b"a", at(1)).expect("put");
    let names: Vec<String> = fs::read_dir(entry.path.parent().expect("folder"))
        .expect("list")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().all(|n| !n.ends_with(".partial")), "{names:?}");
    assert_eq!(names.len(), 2, "data and description: {names:?}");
}

#[test]
fn eviction_removes_the_least_recently_used_first_until_it_fits() {
    let (_dir, store) = store();
    let mut entries = Vec::new();
    for (i, name) in ["old", "middle", "new"].iter().enumerate() {
        let entry = store.put(&format!("https://example.com/{name}.png"), PNG, &[0u8; 100], at(10)).expect("put");
        let file = fs::OpenOptions::new().append(true).open(&entry.path).expect("open");
        file.set_modified(at(1_000 + i as u64)).expect("set time");
        entries.push(entry);
    }
    assert_eq!(store.size(), 300);
    let done = store.evict(250).expect("evict");
    assert_eq!((done.files, done.bytes), (1, 100));
    assert!(!entries[0].path.exists() && entries[1].path.exists() && entries[2].path.exists());
    assert_eq!(store.get("https://example.com/old.png"), None, "its description went with it");
    assert_eq!(store.evict(250).expect("evict"), Evicted::default(), "already small enough");
}

#[test]
fn touching_a_file_protects_it_from_eviction() {
    let (_dir, store) = store();
    let a = store.put("https://example.com/a.png", PNG, &[0u8; 100], at(1)).expect("put");
    let b = store.put("https://example.com/b.png", PNG, &[0u8; 100], at(1)).expect("put");
    for (entry, secs) in [(&a, 100), (&b, 200)] {
        fs::OpenOptions::new().append(true).open(&entry.path).expect("open").set_modified(at(secs)).expect("time");
    }
    store.touch(&a, at(900));
    store.evict(100).expect("evict");
    assert!(a.path.exists() && !b.path.exists(), "b was used least recently");
}

#[test]
fn eviction_sweeps_up_leftovers() {
    let (_dir, store) = store();
    let entry = store.put("https://example.com/a.png", PNG, b"a", at(1)).expect("put");
    let folder = entry.path.parent().expect("folder").to_path_buf();
    fs::write(folder.join("deadbeef00000000.partial"), b"half").expect("partial");
    fs::write(folder.join("deadbeef00000001.png"), b"orphan").expect("orphan");
    let done = store.evict(u64::MAX).expect("evict");
    assert_eq!(done.files, 2);
    assert!(entry.path.exists(), "the real entry stays");
}

#[test]
fn an_empty_or_missing_cache_is_zero_bytes() {
    let (_dir, store) = store();
    assert_eq!(store.size(), 0);
    fs::create_dir_all(store.root()).expect("create");
    assert_eq!(store.size(), 0);
    assert_eq!(store.evict(0).expect("evict"), Evicted::default());
}

#[test]
fn names_are_stable() {
    // These names are on disk in people's caches; a change here orphans them all.
    assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
}
