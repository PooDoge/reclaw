//! Rules that keep the code easy to change, for people and for AI assistants alike: small files
//! with one job, and a map at the top of every module.
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A file past this is split by responsibility (see ARCHITECTURE.md).
const MAX_LINES: usize = 1000;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the crate sits in the workspace").to_path_buf()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(&*name, "target" | ".git" | "node_modules" | "vendor") {
                rust_files(&path, out);
            }
        } else if name.ends_with(".rs") {
            out.push(path);
        }
    }
}

fn all_rust_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    rust_files(&workspace_root(), &mut files);
    files.sort();
    assert!(files.len() > 50, "found only {} files: is the scan looking in the right place?", files.len());
    files
}

fn show(path: &Path) -> String {
    path.strip_prefix(workspace_root()).unwrap_or(path).display().to_string()
}

#[test]
fn no_rust_file_is_longer_than_the_limit() {
    let too_long: Vec<String> = all_rust_files()
        .into_iter()
        .filter_map(|path| {
            let lines = fs::read_to_string(&path).ok()?.lines().count();
            (lines > MAX_LINES).then(|| format!("{} has {lines} lines", show(&path)))
        })
        .collect();
    assert!(too_long.is_empty(), "split these by responsibility (limit {MAX_LINES}):\n{}", too_long.join("\n"));
}

#[test]
fn every_module_root_says_what_the_module_is_for() {
    let missing: Vec<String> = all_rust_files()
        .into_iter()
        .filter(|path| matches!(path.file_name().and_then(|n| n.to_str()), Some("mod.rs" | "lib.rs")))
        .filter(|path| !fs::read_to_string(path).is_ok_and(|text| text.trim_start().starts_with("//!")))
        .map(|path| show(&path))
        .collect();
    assert!(missing.is_empty(), "start these with a //! comment naming the module's job:\n{}", missing.join("\n"));
}

#[test]
fn tests_do_not_share_files_with_a_large_amount_of_code() {
    // A `#[cfg(test)] mod tests` block is fine while small; past this it moves to a `tests/` file or
    // a sibling `tests.rs`, so the code and its tests can each be read on their own.
    const MAX_INLINE_TEST_LINES: usize = 150;
    let mut heavy = Vec::new();
    for path in all_rust_files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let Some(start) = text.find("#[cfg(test)]\nmod tests {") else { continue };
        let test_lines = text[start..].lines().count();
        if test_lines > MAX_INLINE_TEST_LINES {
            heavy.push(format!("{} has {test_lines} lines of inline tests", show(&path)));
        }
    }
    assert!(heavy.is_empty(), "move these tests to their own file:\n{}", heavy.join("\n"));
}

// ---- documentation ----------------------------------------------------------------------------------

/// Every file in the workspace that could be a spec's owner, relative to the root, as path segments.
fn workspace_files() -> Vec<Vec<String>> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<Vec<String>>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !matches!(name.as_str(), "target" | "target-bazzite" | ".git" | "node_modules" | "vendor" | "__pycache__") {
                    walk(&path, root, out);
                }
            } else if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect());
            }
        }
    }
    let mut files = Vec::new();
    walk(&workspace_root(), &workspace_root(), &mut files);
    files
}

/// `*` within a name; true when `text` fits `pattern`.
fn name_matches(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, tail)) => text
            .strip_prefix(head)
            .is_some_and(|rest| (0..=rest.len()).filter(|i| rest.is_char_boundary(*i)).any(|i| name_matches(tail, &rest[i..]))),
    }
}

/// `**` matches any number of folders (none included); `*` matches within one name.
fn glob_matches(pattern: &[&str], path: &[String]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| glob_matches(rest, &path[skip..])),
        Some((first, rest)) => path.split_first().is_some_and(|(name, tail)| name_matches(first, name) && glob_matches(rest, tail)),
    }
}

fn markdown_files(dir: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(workspace_root().join(dir)) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "md")).collect();
    files.sort();
    files
}

fn header_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().take(8).find_map(|line| line.strip_prefix(&format!("- {key}:")).map(str::trim))
}

#[test]
fn every_spec_says_when_it_was_checked_and_names_owners_that_exist() {
    let files = workspace_files();
    let mut problems = Vec::new();
    for spec in markdown_files("docs/specs") {
        let name = show(&spec);
        let text = fs::read_to_string(&spec).unwrap_or_default();
        match header_value(&text, "last-verified") {
            Some(date)
                if date.len() == 10
                    && date.bytes().enumerate().all(|(i, b)| if matches!(i, 4 | 7) { b == b'-' } else { b.is_ascii_digit() }) => {}
            other => problems.push(format!("{name}: `- last-verified: YYYY-MM-DD` is missing or malformed ({other:?})")),
        }
        let Some(owners) = header_value(&text, "owner-paths") else {
            problems.push(format!("{name}: `- owner-paths:` is missing"));
            continue;
        };
        for owner in owners.split(',').map(str::trim).filter(|o| !o.is_empty()) {
            let pattern: Vec<&str> = owner.split('/').collect();
            if !files.iter().any(|f| glob_matches(&pattern, f)) {
                problems.push(format!("{name}: owner path `{owner}` matches no file"));
            }
        }
    }
    assert!(problems.is_empty(), "docs/README.md rules 2:\n{}", problems.join("\n"));
}

#[test]
fn decision_records_are_numbered_once_and_say_what_became_of_them() {
    let mut problems = Vec::new();
    let mut numbers = std::collections::HashSet::new();
    for adr in markdown_files("docs/adr") {
        let name = show(&adr);
        let file = adr.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let number = file.get(..4).filter(|n| n.bytes().all(|b| b.is_ascii_digit()) && file.get(4..5) == Some("-"));
        match number {
            Some(n) if numbers.insert(n.to_string()) => {}
            Some(n) => problems.push(format!("{name}: number {n} is used twice")),
            None => problems.push(format!("{name}: the name must start with a four-digit number and a dash")),
        }
        let text = fs::read_to_string(&adr).unwrap_or_default();
        match header_value(&text, "status") {
            Some(status) if status == "accepted" || status.starts_with("superseded by ") => {}
            other => problems.push(format!("{name}: `- status: accepted` or `- status: superseded by NNNN` is missing ({other:?})")),
        }
    }
    assert!(problems.is_empty(), "docs/README.md rules 4:\n{}", problems.join("\n"));
}

#[test]
fn the_glob_matcher_follows_its_own_rules() {
    let path = |s: &str| s.split('/').map(String::from).collect::<Vec<_>>();
    let matches = |pattern: &str, p: &str| glob_matches(&pattern.split('/').collect::<Vec<_>>(), &path(p));
    assert!(matches("a/**", "a/b/c.rs"));
    assert!(matches("a/**/c.rs", "a/c.rs"));
    assert!(matches("a/*.rs", "a/x.rs") && !matches("a/*.rs", "a/b/x.rs"));
    assert!(matches("a/b.rs", "a/b.rs") && !matches("a/b.rs", "a/c.rs"));
    assert!(!matches("a/**", "b/c.rs"));
}

/// The Freya release that the docs' claims about the toolkit were checked against (AGENTS.md, "Freya
/// source of truth"). Moving to another release means re-checking those claims first.
const FREYA_RELEASE: &str = "0.5.0-rc.8";

#[test]
fn freya_stays_on_the_release_the_docs_were_checked_against() {
    let root = workspace_root();
    let lock = fs::read_to_string(root.join("Cargo.lock")).expect("Cargo.lock is committed");
    let mut checked = Vec::new();
    for package in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            package
                .lines()
                .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(" = \"")).and_then(|r| r.strip_suffix('"')))
                .unwrap_or_default()
        };
        let name = field("name");
        // The graphics bindings (`freya-skia-*`) are versioned on their own.
        if !(name == "freya" || name.starts_with("freya-")) || name.starts_with("freya-skia") {
            continue;
        }
        assert_eq!(field("version"), FREYA_RELEASE, "{name} is not at the release the docs were checked against");
        assert!(field("source").starts_with("registry+"), "{name} must come from crates.io, not a path or a git checkout");
        checked.push(name.to_string());
    }
    assert!(
        checked.len() >= 10 && checked.iter().any(|n| n == "freya") && checked.iter().any(|n| n == "freya-core"),
        "Cargo.lock should list Freya's crates, found {checked:?}"
    );

    let mut manifests = vec![root.join("Cargo.toml")];
    manifests.extend(
        fs::read_dir(&root).expect("the workspace root is readable").flatten().map(|e| e.path().join("Cargo.toml")).filter(|p| p.is_file()),
    );
    for manifest in manifests {
        let text = fs::read_to_string(&manifest).expect("a manifest is readable");
        assert!(!text.contains("[patch"), "{}: a [patch] can swap Freya for source the docs were not checked against", show(&manifest));
        for line in text.lines().filter(|l| l.trim_start().starts_with("freya")) {
            assert!(line.contains(&format!("\"={FREYA_RELEASE}\"")), "{}: pin Freya exactly with `=`: {line}", show(&manifest));
        }
    }
}
