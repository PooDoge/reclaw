use super::*;

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(dir).expect("read dir").flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

fn line(n: usize) -> Vec<u8> {
    format!("line {n:04} {}\n", "x".repeat(40)).into_bytes()
}

#[test]
fn lines_are_appended_and_a_second_open_continues_the_same_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    RotatingFile::open(dir.path(), "reclaw", 10_000, 3).expect("open").write_all(b"first\n").expect("write");
    RotatingFile::open(dir.path(), "reclaw", 10_000, 3).expect("reopen").write_all(b"second\n").expect("write");
    assert_eq!(fs::read_to_string(dir.path().join("reclaw.log")).expect("read"), "first\nsecond\n");
}

#[test]
fn a_full_file_is_moved_aside_and_only_keep_files_remain() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut log = RotatingFile::open(dir.path(), "reclaw", 200, 3).expect("open");
    for n in 0..40 {
        log.write_all(&line(n)).expect("write");
    }
    assert_eq!(names(dir.path()), ["reclaw.1.log", "reclaw.2.log", "reclaw.log"], "three files, however many lines");
    let newest = fs::read_to_string(dir.path().join("reclaw.log")).expect("read");
    assert!(newest.contains("line 0039"), "the newest line is in the current file");
    let older = fs::read_to_string(dir.path().join("reclaw.1.log")).expect("read");
    let oldest = fs::read_to_string(dir.path().join("reclaw.2.log")).expect("read");
    let first_in = |text: &str| text.lines().next().map(str::to_string).unwrap_or_default();
    assert!(first_in(&older) > first_in(&oldest), "1 is newer than 2: {} vs {}", first_in(&older), first_in(&oldest));
    for file in [&newest, &older, &oldest] {
        assert!(file.len() <= 200 + 60, "a file stays near the limit, and a line is never split: {}", file.len());
        assert!(file.lines().all(|l| l.starts_with("line ")), "no line is cut in two");
    }
}

#[test]
fn a_file_already_past_the_limit_is_moved_aside_at_start() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("reclaw.log"), "old ".repeat(100)).expect("seed");
    let mut log = RotatingFile::open(dir.path(), "reclaw", 100, 2).expect("open");
    log.write_all(b"new run\n").expect("write");
    assert_eq!(fs::read_to_string(dir.path().join("reclaw.log")).expect("read"), "new run\n");
    assert!(fs::read_to_string(dir.path().join("reclaw.1.log")).expect("read").starts_with("old"));
}

#[test]
fn keeping_one_file_discards_the_old_one_instead_of_growing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut log = RotatingFile::open(dir.path(), "reclaw", 100, 1).expect("open");
    for n in 0..20 {
        log.write_all(&line(n)).expect("write");
    }
    assert_eq!(names(dir.path()), ["reclaw.log"]);
}

#[test]
fn a_line_longer_than_the_limit_is_written_whole() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut log = RotatingFile::open(dir.path(), "reclaw", 50, 2).expect("open");
    let long = vec![b'a'; 500];
    log.write_all(&long).expect("write");
    assert_eq!(fs::metadata(dir.path().join("reclaw.log")).expect("meta").len(), 500, "nothing is lost to the limit");
}

#[test]
fn a_folder_that_cannot_be_made_is_an_error_not_a_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blocker = dir.path().join("file");
    fs::write(&blocker, "x").expect("seed");
    assert!(RotatingFile::open(&blocker.join("logs"), "reclaw", 100, 2).is_err());
}
