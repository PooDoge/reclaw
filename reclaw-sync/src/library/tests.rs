use reclaw_catalog::AppEntry;

use super::*;

fn app(name: &str, repo: &str) -> AppEntry {
    AppEntry {
        name: name.into(),
        repository: repo.into(),
        folder_name: name.replace(' ', ""),
        tags: vec!["recomp".into()],
        ..Default::default()
    }
}

fn store(dir: &tempfile::TempDir) -> LibraryStore {
    LibraryStore::new(dir.path().join("data").join("apps.json"))
}

#[test]
fn a_first_run_has_an_empty_library() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(store(&dir).load(), Ok(vec![]));
}

#[test]
fn what_is_saved_loads_back_and_the_folder_is_made() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    let apps = vec![app("Super Mario 64", "o/sm64"), app("Mine", "")];
    store.save(&apps).expect("saved");
    assert_eq!(store.load(), Ok(apps));
}

#[test]
fn a_library_that_cannot_be_read_is_an_error_and_is_left_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    fs::create_dir_all(store.path().parent().expect("dir")).expect("mkdir");
    for damaged in ["", "{", "[1,2", r#"{"apps": [{"name": 5}]}"#, "null"] {
        fs::write(store.path(), damaged).expect("write");
        assert!(matches!(store.load(), Err(LibraryError::Corrupt { .. })), "{damaged:?}");
        assert_eq!(fs::read_to_string(store.path()).expect("read"), damaged, "reading never changes the file");
    }
}

#[test]
fn replacing_a_library_keeps_a_copy_of_the_old_one_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    store.save(&[app("One", "o/one")]).expect("first");
    let old = fs::read(store.path()).expect("old");
    store.save(&[app("One", "o/one"), app("Two", "o/two")]).expect("second");
    store.save(&[app("One", "o/one"), app("Two", "o/two")]).expect("the same again");
    let backups: Vec<PathBuf> =
        fs::read_dir(store.path().with_file_name("backups")).expect("backups").flatten().map(|e| e.path()).collect();
    assert_eq!(backups.len(), 1, "the unchanged save made no copy: {backups:?}");
    assert_eq!(fs::read(&backups[0]).expect("backup"), old);
}

#[test]
fn only_the_newest_backups_are_kept() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    for i in 0..(KEEP_BACKUPS + 6) {
        store.save(&[app(&format!("Game {i}"), &format!("o/g{i}"))]).expect("save");
    }
    let count = fs::read_dir(store.path().with_file_name("backups")).expect("backups").count();
    assert_eq!(count, KEEP_BACKUPS);
}

#[test]
fn no_temporary_file_is_left_behind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    store.save(&[app("One", "o/one")]).expect("saved");
    let mut names: Vec<String> = fs::read_dir(store.path().parent().expect("dir"))
        .expect("ls")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["apps.json", "apps.json.lock"]);
}

#[test]
fn two_copies_cannot_save_at_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    store.save(&[app("One", "o/one")]).expect("first");
    // Another copy of the program holds the lock.
    let other = File::create(store.path().with_file_name("apps.json.lock")).expect("lock file");
    other.lock().expect("held");
    assert_eq!(store.save(&[app("Two", "o/two")]), Err(LibraryError::Locked));
    assert_eq!(store.load().expect("still the first").len(), 1);
    drop(other);
    store.save(&[app("Two", "o/two")]).expect("free again");
}

#[test]
fn a_damaged_library_can_be_moved_aside_on_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    fs::create_dir_all(store.path().parent().expect("dir")).expect("mkdir");
    fs::write(store.path(), "{ not json").expect("write");
    let moved = store.set_aside().expect("moved");
    assert!(moved.exists() && !store.path().exists());
    assert_eq!(fs::read_to_string(&moved).expect("read"), "{ not json", "nothing is lost");
    assert_eq!(store.load(), Ok(vec![]), "and a new library can start");
}

#[test]
fn unreadable_is_not_the_same_as_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store(&dir);
    // A directory where the file should be.
    fs::create_dir_all(store.path()).expect("mkdir");
    assert!(matches!(store.load(), Err(LibraryError::Unreadable { .. })));
}
