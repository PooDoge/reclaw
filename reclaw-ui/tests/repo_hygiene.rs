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
