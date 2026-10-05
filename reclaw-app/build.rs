//! Stamps the build with what a bug report needs: which commit it was built from, which profile, and where the source checkout
//! is (so a development build can update itself from it). Each is "unknown" or empty when it cannot be found, never an error:
//! a build from a source archive has no `.git`.
use std::{path::Path, process::Command};

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().unwrap_or(manifest);
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let sha = git(&["rev-parse", "--short=10", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
    println!("cargo:rustc-env=RECLAW_GIT_SHA={sha}{}", if dirty { "+local-changes" } else { "" });
    println!("cargo:rustc-env=RECLAW_PROFILE={}", std::env::var("PROFILE").unwrap_or_default());
    // Only a checkout (a folder with a .git) counts as a place to update from.
    let source = if root.join(".git").exists() { root.display().to_string() } else { String::new() };
    println!("cargo:rustc-env=RECLAW_SOURCE_DIR={source}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
    println!("cargo:rerun-if-changed=build.rs");
}
