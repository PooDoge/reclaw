//! `scripts/update.sh` against real git repositories made for the test: a remote, a second clone that pushes, and a checkout that
//! updates. The script must follow the remote when that is safe and otherwise change nothing at all.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("workspace").join("scripts").join("update.sh")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "t@example.org")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "t@example.org")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

struct World {
    _dir: tempfile::TempDir,
    remote: PathBuf,
    /// Another person's clone, which pushes.
    other: PathBuf,
    /// The checkout under test, with the script in `scripts/`.
    checkout: PathBuf,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let (remote, other, checkout) = (dir.path().join("remote.git"), dir.path().join("other"), dir.path().join("checkout"));
        fs::create_dir_all(&remote).expect("mkdir");
        git(&remote, &["init", "--bare", "--initial-branch=main", "."]);
        git(dir.path(), &["clone", "--quiet", remote.to_str().expect("utf8"), "other"]);
        git(&other, &["checkout", "-b", "main"]);
        fs::create_dir_all(other.join("scripts")).expect("mkdir");
        fs::copy(script(), other.join("scripts").join("update.sh")).expect("copy the script");
        fs::write(other.join("README"), "one\n").expect("write");
        git(&other, &["add", "."]);
        git(&other, &["commit", "--quiet", "-m", "first"]);
        git(&other, &["push", "--quiet", "-u", "origin", "main"]);
        git(dir.path(), &["clone", "--quiet", remote.to_str().expect("utf8"), "checkout"]);
        Self { _dir: dir, remote, other, checkout }
    }

    fn push(&self, file: &str, text: &str, message: &str) {
        fs::write(self.other.join(file), text).expect("write");
        git(&self.other, &["add", "."]);
        git(&self.other, &["commit", "--quiet", "-m", message]);
        git(&self.other, &["push", "--quiet"]);
    }

    fn head(&self) -> String {
        git(&self.checkout, &["rev-parse", "--short=10", "HEAD"])
    }

    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut command = Command::new("bash");
        command.arg(self.checkout.join("scripts").join("update.sh")).args(args).current_dir(&self.checkout);
        command.env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_SYSTEM", "/dev/null").env_remove("RECLAW_BUILD_COMMAND");
        for (k, v) in env {
            command.env(k, v);
        }
        command.output().expect("bash runs")
    }
}

fn text(out: &Output) -> (String, String) {
    (String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// The program's line: `RECLAW_UPDATE: <result> <old> <new> <binary>`.
fn marker(out: &Output) -> Vec<String> {
    let (stdout, _) = text(out);
    let line = stdout.lines().rev().find(|l| l.starts_with("RECLAW_UPDATE: ")).unwrap_or_else(|| panic!("no result line in:\n{stdout}"));
    line.trim_start_matches("RECLAW_UPDATE: ").split(' ').map(str::to_string).collect()
}

#[test]
fn a_checkout_that_is_current_says_so_and_changes_nothing() {
    let w = World::new();
    let before = w.head();
    let out = w.run(&["--no-build"], &[]);
    assert!(out.status.success(), "{:?}", text(&out));
    assert!(text(&out).0.contains("already up to date"), "{:?}", text(&out));
    assert_eq!(marker(&out), ["up-to-date", before.as_str(), before.as_str(), "none"].map(String::from));
}

#[test]
fn new_commits_are_listed_and_fast_forwarded() {
    let w = World::new();
    let before = w.head();
    w.push("a.txt", "a\n", "add the thing");
    w.push("b.txt", "b\n", "fix the other thing");
    let out = w.run(&["--no-build"], &[]);
    assert!(out.status.success(), "{:?}", text(&out));
    let (stdout, _) = text(&out);
    assert!(stdout.contains("add the thing") && stdout.contains("fix the other thing"), "the new commits are shown:\n{stdout}");
    assert!(w.checkout.join("b.txt").exists(), "the files arrived");
    let after = w.head();
    assert_ne!(before, after);
    assert_eq!(marker(&out), ["updated", before.as_str(), after.as_str(), "none"].map(String::from));
}

#[test]
fn check_only_reports_and_changes_nothing() {
    let w = World::new();
    let before = w.head();
    w.push("a.txt", "a\n", "something new");
    let out = w.run(&["--check"], &[]);
    assert!(out.status.success(), "{:?}", text(&out));
    assert_eq!(marker(&out)[0], "available");
    assert_eq!(w.head(), before);
    assert!(!w.checkout.join("a.txt").exists());
}

#[test]
fn local_changes_to_tracked_files_stop_it_before_anything_happens_but_untracked_files_do_not() {
    let w = World::new();
    w.push("a.txt", "a\n", "something new");
    let before = w.head();
    fs::write(w.checkout.join("scratch.txt"), "mine\n").expect("write");
    let ok = w.run(&["--no-build"], &[]);
    assert!(ok.status.success(), "an untracked file is not in the way: {:?}", text(&ok));
    assert_ne!(w.head(), before);

    w.push("c.txt", "c\n", "and again");
    fs::write(w.checkout.join("README"), "edited by me\n").expect("write");
    let head = w.head();
    let out = w.run(&["--no-build"], &[]);
    assert_eq!(out.status.code(), Some(4), "{:?}", text(&out));
    assert_eq!(w.head(), head);
    assert_eq!(fs::read_to_string(w.checkout.join("README")).expect("read"), "edited by me\n", "the edit is untouched");
    assert!(text(&out).1.contains("README"), "it says which file: {:?}", text(&out));
    assert_eq!(marker(&out)[0], "blocked-local-changes");
}

#[test]
fn histories_that_have_both_moved_are_never_merged_or_reset() {
    let w = World::new();
    fs::write(w.checkout.join("mine.txt"), "mine\n").expect("write");
    git(&w.checkout, &["add", "."]);
    git(&w.checkout, &["commit", "--quiet", "-m", "my own work"]);
    w.push("theirs.txt", "theirs\n", "their work");
    let head = w.head();
    let out = w.run(&["--no-build"], &[]);
    assert_eq!(out.status.code(), Some(3), "{:?}", text(&out));
    assert_eq!(w.head(), head, "nothing moved");
    assert!(w.checkout.join("mine.txt").exists() && !w.checkout.join("theirs.txt").exists());
    assert_eq!(marker(&out)[0], "cannot-fast-forward");
}

#[test]
fn a_checkout_with_commits_the_remote_lacks_is_left_alone() {
    let w = World::new();
    fs::write(w.checkout.join("mine.txt"), "mine\n").expect("write");
    git(&w.checkout, &["add", "."]);
    git(&w.checkout, &["commit", "--quiet", "-m", "my own work"]);
    let head = w.head();
    let out = w.run(&["--no-build"], &[]);
    assert!(out.status.success(), "{:?}", text(&out));
    assert!(text(&out).0.contains("ahead of"), "{:?}", text(&out));
    assert_eq!((w.head(), marker(&out)[0].as_str()), (head, "ahead"));
}

#[test]
fn a_branch_with_no_upstream_or_a_detached_head_cannot_be_updated_and_says_how_to_fix_it() {
    let w = World::new();
    git(&w.checkout, &["checkout", "--quiet", "-b", "topic"]);
    let out = w.run(&["--no-build"], &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out).1.contains("set-upstream-to"), "{:?}", text(&out));
    git(&w.checkout, &["checkout", "--quiet", "--detach"]);
    let out = w.run(&["--no-build"], &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out).1.contains("detached"), "{:?}", text(&out));
}

#[test]
fn an_unreachable_remote_is_reported_as_a_fetch_failure() {
    let w = World::new();
    git(&w.checkout, &["remote", "set-url", "origin", w.remote.join("nope").to_str().expect("utf8")]);
    let before = w.head();
    let out = w.run(&["--no-build"], &[]);
    assert_eq!(out.status.code(), Some(5), "{:?}", text(&out));
    assert_eq!((w.head(), marker(&out)[0].as_str()), (before, "fetch-failed"));
}

#[test]
fn the_build_runs_after_the_update_and_its_result_names_the_binary() {
    let w = World::new();
    w.push("a.txt", "a\n", "something new");
    let target = w.checkout.join("target");
    let build = format!("mkdir -p {0}/release && echo built > {0}/release/reclaw && chmod +x {0}/release/reclaw", target.display());
    let out = w.run(&[], &[("RECLAW_BUILD_COMMAND", &build), ("CARGO_TARGET_DIR", target.to_str().expect("utf8"))]);
    assert!(out.status.success(), "{:?}", text(&out));
    let result = marker(&out);
    assert_eq!(result[0], "updated");
    assert_eq!(result[3], format!("{}/release/reclaw", target.display()));
    assert!(text(&out).0.contains("building:"));
}

#[test]
fn a_failed_build_is_reported_with_the_source_already_updated() {
    let w = World::new();
    w.push("a.txt", "a\n", "something new");
    let before = w.head();
    let out = w.run(&[], &[("RECLAW_BUILD_COMMAND", "echo compile error >&2; exit 1")]);
    assert_eq!(out.status.code(), Some(6), "{:?}", text(&out));
    assert_ne!(w.head(), before, "the new source is in place");
    assert_eq!(marker(&out)[0], "build-failed");
    assert!(text(&out).1.contains("compile error"), "the build's own message is passed on: {:?}", text(&out));
}

#[test]
fn a_build_that_claims_success_but_leaves_no_binary_is_a_failure() {
    let w = World::new();
    let target = w.checkout.join("empty-target");
    let out = w.run(&[], &[("RECLAW_BUILD_COMMAND", "true"), ("CARGO_TARGET_DIR", target.to_str().expect("utf8"))]);
    assert_eq!(out.status.code(), Some(6), "{:?}", text(&out));
}

#[test]
fn the_profile_chooses_which_binary_is_expected() {
    let w = World::new();
    let target = w.checkout.join("t");
    let build = format!("mkdir -p {0}/debug && touch {0}/debug/reclaw && chmod +x {0}/debug/reclaw", target.display());
    let out = w.run(&["--profile", "debug"], &[("RECLAW_BUILD_COMMAND", &build), ("CARGO_TARGET_DIR", target.to_str().expect("utf8"))]);
    assert!(out.status.success(), "{:?}", text(&out));
    assert!(marker(&out)[3].ends_with("/debug/reclaw"));
    assert_eq!(w.run(&["--profile", "fast"], &[]).status.code(), Some(2));
}
