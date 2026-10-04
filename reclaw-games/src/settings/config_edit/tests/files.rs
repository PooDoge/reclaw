use super::*;
use crate::settings::testing::TempDir;
use std::fs;

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(dir).expect("read dir").map(|e| e.expect("entry").file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn creates_the_file_and_its_directories() {
    let dir = TempDir::new();
    let path = dir.path().join("deep/er/graphics.json");
    write_atomic(&path, "{}").expect("write");
    assert_eq!(fs::read_to_string(&path).expect("read"), "{}");
    assert_eq!(names(path.parent().expect("parent")), ["graphics.json"], "no temporary file and no backup for a new file");
}

#[test]
fn replaces_the_content() {
    let dir = TempDir::new();
    let path = dir.path().join("a.ini");
    fs::write(&path, "old content that is longer").expect("seed");
    write_atomic(&path, "new").expect("write");
    assert_eq!(fs::read_to_string(&path).expect("read"), "new");
}

#[test]
fn keeps_only_the_first_original() {
    let dir = TempDir::new();
    let path = dir.path().join("a.ini");
    fs::write(&path, "original").expect("seed");
    write_atomic(&path, "first edit").expect("write");
    write_atomic(&path, "second edit").expect("write");
    assert_eq!(fs::read_to_string(dir.path().join("a.ini.reclaw-orig")).expect("backup"), "original");
    assert_eq!(fs::read_to_string(&path).expect("read"), "second edit");
    assert_eq!(names(dir.path()), ["a.ini", "a.ini.reclaw-orig"]);
}

#[test]
fn an_existing_backup_is_never_overwritten() {
    let dir = TempDir::new();
    let path = dir.path().join("a.ini");
    fs::write(&path, "current").expect("seed");
    fs::write(dir.path().join("a.ini.reclaw-orig"), "someone else's").expect("seed backup");
    write_atomic(&path, "new").expect("write");
    assert_eq!(fs::read_to_string(dir.path().join("a.ini.reclaw-orig")).expect("backup"), "someone else's");
}

#[cfg(unix)]
#[test]
fn permissions_of_the_existing_file_are_kept() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new();
    let path = dir.path().join("secret.cfg");
    fs::write(&path, "x").expect("seed");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("chmod");
    write_atomic(&path, "y").expect("write");
    assert_eq!(fs::metadata(&path).expect("meta").permissions().mode() & 0o777, 0o640);
}

#[test]
fn a_failed_rename_leaves_nothing_behind() {
    let dir = TempDir::new();
    let target = dir.path().join("taken");
    fs::create_dir(&target).expect("a directory where the file should go");
    let err = write_atomic(&target, "x").expect_err("a directory cannot be replaced by a file");
    assert!(matches!(err, ConfigEditError::Io { .. }), "{err}");
    assert_eq!(names(dir.path()), ["taken"], "the temporary file is removed");
}

#[test]
fn a_path_without_a_file_name_is_an_error() {
    assert!(matches!(write_atomic(std::path::Path::new(".."), "x"), Err(ConfigEditError::BadPath { .. })));
}

#[test]
fn a_parent_that_is_a_file_is_an_io_error() {
    let dir = TempDir::new();
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "x").expect("seed");
    assert!(matches!(write_atomic(&blocker.join("child.ini"), "x"), Err(ConfigEditError::Io { .. })));
}

#[test]
fn parallel_writes_to_one_file_never_corrupt_it() {
    let dir = TempDir::new();
    let path = dir.path().join("shared.json");
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let path = path.clone();
            std::thread::spawn(move || write_atomic(&path, &format!("writer {i} wrote this whole file")))
        })
        .collect();
    for handle in handles {
        handle.join().expect("thread").expect("write");
    }
    let content = fs::read_to_string(&path).expect("read");
    assert!(content.starts_with("writer ") && content.ends_with(" wrote this whole file"), "{content}");
    assert!(names(dir.path()).iter().all(|n| !n.contains("tmp")), "{:?}", names(dir.path()));
}
