use super::*;

fn entry(url: &str, body: &[u8], at: u64) -> Entry {
    Entry {
        meta: Meta {
            url: url.into(),
            etag: Some("\"abc\"".into()),
            last_modified: None,
            content_type: Some("application/json".into()),
            fetched_at: at,
        },
        body: body.to_vec(),
    }
}

#[test]
fn what_is_written_reads_back_whole_including_binary_bodies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = FileCache::new(dir.path());
    let body = [0u8, b'\n', 255, b'\n', 7];
    cache.write("https://e.test/a", "anon", &entry("https://e.test/a", &body, 100)).expect("write");
    let back = cache.read("https://e.test/a", "anon").expect("hit");
    assert_eq!(back.body, body);
    assert_eq!(back.meta.etag.as_deref(), Some("\"abc\""));
    assert_eq!(back.age_secs(160), 60);
    assert_eq!(back.age_secs(50), 0, "a clock that went backwards is not a negative age");
}

#[test]
fn another_identity_or_address_does_not_see_the_copy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = FileCache::new(dir.path());
    cache.write("https://e.test/a", "anon", &entry("https://e.test/a", b"x", 1)).expect("write");
    assert!(
        cache.read("https://e.test/a", "6f00aa11bb22").is_none(),
        "an authenticated request must not get the anonymous answer, nor the reverse"
    );
    assert!(cache.read("https://e.test/b", "anon").is_none());
}

#[test]
fn a_damaged_file_is_a_miss() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = FileCache::new(dir.path());
    cache.write("https://e.test/a", "anon", &entry("https://e.test/a", b"x", 1)).expect("write");
    let file = fs::read_dir(dir.path()).expect("dir").flatten().next().expect("one file").path();
    for damage in [&b""[..], b"RNC1\n", b"RNC1\nnot json\nbody", b"garbage"] {
        fs::write(&file, damage).expect("damage");
        assert!(cache.read("https://e.test/a", "anon").is_none(), "{damage:?}");
    }
}

#[test]
fn a_write_replaces_and_leaves_no_temporary_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = FileCache::new(dir.path().join("nested"));
    cache.write("https://e.test/a", "anon", &entry("https://e.test/a", b"one", 1)).expect("first");
    cache.write("https://e.test/a", "anon", &entry("https://e.test/a", b"two", 2)).expect("second");
    assert_eq!(cache.read("https://e.test/a", "anon").map(|e| e.body), Some(b"two".to_vec()));
    let names: Vec<String> =
        fs::read_dir(cache.dir()).expect("dir").flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 1, "{names:?}");
}
