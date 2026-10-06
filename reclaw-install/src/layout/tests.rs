use std::{
    fs,
    path::{Path, PathBuf},
};

use super::*;
use crate::{platform::Platform, programs};

const LINUX: Platform = Platform::LinuxX64;

fn elf() -> Vec<u8> {
    let mut header = vec![0u8; 64];
    header[..4].copy_from_slice(b"\x7fELF");
    header[4] = 2;
    header[5] = 1;
    header[6] = 1;
    header[16] = 2;
    header[24] = 0x10;
    header
}

fn put(root: &Path, path: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let full = root.join(path);
    fs::create_dir_all(full.parent().expect("parent")).expect("dirs");
    fs::write(&full, content).expect("file");
    full
}

fn names(root: &Path, found: &programs::Programs) -> Vec<String> {
    found.programs.iter().map(|p| p.strip_prefix(root).expect("inside").to_string_lossy().into_owned()).collect()
}

#[test]
fn metadata_files_are_the_launchers_own() {
    for name in ["version.txt", "VERSION.TXT", "install-incomplete.txt", "LastPlayed.txt", "selected_executable.txt"] {
        assert!(is_metadata_file(name), "{name}");
    }
    assert!(!is_metadata_file("readme.txt") && !is_metadata_file("game.x86_64"));
}

#[test]
fn normalizing_resolves_dots_without_the_disk() {
    assert_eq!(normalize(Path::new("/a/b/../c/./d")), PathBuf::from("/a/c/d"));
    assert_eq!(normalize(Path::new("a/../../b")), PathBuf::from("../b"));
}

#[test]
fn a_wrapper_folder_is_hoisted_once_a_program_is_inside() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "Game-1.0/game.x86_64", elf());
    put(dir.path(), "Game-1.0/data/a.txt", "a");
    hoist_wrapper(dir.path(), LINUX).expect("hoisted");
    assert!(dir.path().join("game.x86_64").is_file() && dir.path().join("data/a.txt").is_file());
    assert!(!dir.path().join("Game-1.0").exists());
}

#[test]
fn two_wrappers_deep_are_both_hoisted() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "outer/inner/game.x86_64", elf());
    hoist_wrapper(dir.path(), LINUX).expect("hoisted");
    assert!(dir.path().join("game.x86_64").is_file());
}

#[test]
fn a_wrapper_with_the_name_of_something_inside_it_is_hoisted_too() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "Game/Game/data.txt", "d");
    put(dir.path(), "Game/game.x86_64", elf());
    hoist_wrapper(dir.path(), LINUX).expect("hoisted");
    assert!(dir.path().join("game.x86_64").is_file() && dir.path().join("Game/data.txt").is_file());
}

#[test]
fn siblings_of_the_folder_mean_it_is_not_just_a_wrapper() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "Game/game.x86_64", elf());
    put(dir.path(), "extra.toml", "x");
    hoist_wrapper(dir.path(), LINUX).expect("left alone");
    assert!(dir.path().join("Game/game.x86_64").is_file(), "structure kept");

    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "a/game.x86_64", elf());
    put(dir.path(), "b/readme.txt", "x");
    hoist_wrapper(dir.path(), LINUX).expect("left alone");
    assert!(dir.path().join("a/game.x86_64").is_file());
}

#[test]
fn launcher_metadata_beside_the_wrapper_does_not_count_as_a_sibling() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "Game/game.x86_64", elf());
    put(dir.path(), "version.txt", "1");
    hoist_wrapper(dir.path(), LINUX).expect("hoisted");
    assert!(dir.path().join("game.x86_64").is_file());
}

#[test]
fn a_top_level_program_means_nothing_to_hoist() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "game.x86_64", elf());
    put(dir.path(), "only/other.x86_64", elf());
    hoist_wrapper(dir.path(), LINUX).expect("untouched");
    assert!(dir.path().join("only/other.x86_64").is_file());
}

#[test]
fn merging_overlays_and_keeps_what_the_new_tree_lacks() {
    let dir = tempfile::tempdir().expect("dir");
    let (stage, app) = (dir.path().join("stage"), dir.path().join("app"));
    put(&app, "game.x86_64", "old program");
    put(&app, "saves/slot1.sav", "my progress");
    put(&app, "config.ini", "my settings");
    put(&app, "data/old.bin", "old");
    put(&stage, "game.x86_64", "new program");
    put(&stage, "data/new.bin", "new");
    merge_into(&stage, &app).expect("merged");
    assert_eq!(fs::read_to_string(app.join("game.x86_64")).expect("read"), "new program");
    assert_eq!(fs::read_to_string(app.join("saves/slot1.sav")).expect("read"), "my progress");
    assert_eq!(fs::read_to_string(app.join("config.ini")).expect("read"), "my settings");
    assert!(app.join("data/old.bin").is_file() && app.join("data/new.bin").is_file());
}

#[test]
fn a_file_and_a_folder_of_the_same_name_replace_each_other() {
    let dir = tempfile::tempdir().expect("dir");
    let (stage, app) = (dir.path().join("stage"), dir.path().join("app"));
    put(&app, "was_file", "f");
    put(&app, "was_dir/inner.txt", "i");
    put(&stage, "was_file/now_dir.txt", "d");
    put(&stage, "was_dir", "now a file");
    merge_into(&stage, &app).expect("merged");
    assert!(app.join("was_file/now_dir.txt").is_file());
    assert_eq!(fs::read_to_string(app.join("was_dir")).expect("read"), "now a file");
}

#[test]
fn the_version_is_known_only_for_a_finished_install() {
    let dir = tempfile::tempdir().expect("dir");
    assert_eq!(installed_version(dir.path()), None);
    put(dir.path(), VERSION_FILE, " v1.2.3\n");
    assert_eq!(installed_version(dir.path()).as_deref(), Some("v1.2.3"));
    put(dir.path(), INCOMPLETE_FILE, "v1.2.3");
    assert_eq!(installed_version(dir.path()), None, "the marker means it did not finish");
    put(dir.path(), "x/version.txt", "");
    assert_eq!(installed_version(&dir.path().join("x")), None, "an empty file says nothing");
}

#[test]
fn complete_means_finished_and_something_to_start() {
    let dir = tempfile::tempdir().expect("dir");
    assert!(!is_complete(dir.path(), LINUX));
    put(dir.path(), "game.x86_64", elf());
    assert!(is_complete(dir.path(), LINUX));
    put(dir.path(), INCOMPLETE_FILE, "v1");
    assert!(!is_complete(dir.path(), LINUX));
}

#[test]
fn linux_programs_are_found_by_what_they_are_not_what_they_are_called() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    put(root, "Game", elf()); // no extension, no execute bit
    put(root, "LICENSE", "text");
    put(root, "libfoo.so", elf());
    put(root, "libfoo.so.1", elf());
    put(root, "run.sh", "#!/bin/sh\n");
    put(root, "notes.txt", "x");
    put(root, "tool.x86_64", "anything");
    put(root, "deep/dir/helper", elf());
    let found = programs::find(root, true, LINUX);
    // Nearest the top first, then by name; libraries, text and a LICENSE are not programs.
    assert_eq!(names(root, &found), ["Game", "run.sh", "tool.x86_64", "deep/dir/helper"]);
    assert!(!found.needs_runner);
    let top = programs::find(root, false, LINUX);
    assert_eq!(names(root, &top), ["Game", "run.sh", "tool.x86_64"], "a top-level search stays at the top");
}

#[test]
fn windows_programs_count_only_when_nothing_is_native() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "Game.exe", "MZ");
    put(dir.path(), "data/readme.txt", "x");
    let found = programs::find(dir.path(), true, LINUX);
    assert_eq!(names(dir.path(), &found), ["Game.exe"]);
    assert!(found.needs_runner, "a Windows program on Linux needs a compatibility layer");
    put(dir.path(), "native", elf());
    let found = programs::find(dir.path(), true, LINUX);
    assert_eq!(names(dir.path(), &found), ["native"]);
    assert!(!found.needs_runner);
}

#[test]
fn elf_files_deep_in_a_windows_build_do_not_make_it_native() {
    // Digimon World Recompiled's Windows zip ships its sources, test fixtures of an ELF library among them.
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "Digimon_World_Recompiled.exe", "MZ");
    put(dir.path(), "psxrecomp/recompiler/lib/ELFIO/tests/elf_examples/asm64", elf());
    put(dir.path(), "psxrecomp/packaging/make-icons.sh", "#!/bin/sh\n");
    let found = programs::find(dir.path(), true, LINUX);
    assert_eq!(names(dir.path(), &found), ["Digimon_World_Recompiled.exe", "psxrecomp/packaging/make-icons.sh"]);
    assert!(found.needs_runner);
    // A native program as near the top as the .exe still wins.
    put(dir.path(), "Digimon_World_Recompiled", elf());
    let found = programs::find(dir.path(), true, LINUX);
    assert_eq!(found.programs.first(), Some(&dir.path().join("Digimon_World_Recompiled")));
    assert!(!found.needs_runner);
}

#[test]
fn on_windows_exe_bat_and_cmd_are_programs() {
    let dir = tempfile::tempdir().expect("dir");
    for f in ["a.exe", "b.BAT", "c.cmd", "d.dll"] {
        put(dir.path(), f, "x");
    }
    let found = programs::find(dir.path(), true, Platform::Windows);
    assert_eq!(names(dir.path(), &found), ["a.exe", "b.BAT", "c.cmd"]);
}

#[test]
fn a_wine_prefix_and_the_stage_folder_are_not_part_of_the_game() {
    let dir = tempfile::tempdir().expect("dir");
    put(dir.path(), "game.x86_64", elf());
    put(dir.path(), ".wine-prefix/drive_c/windows/notepad.exe", "MZ");
    put(dir.path(), "prefix/drive_c/x.exe", "MZ");
    fs::create_dir_all(dir.path().join("prefix/dosdevices")).expect("dos");
    put(dir.path(), ".reclaw-stage/unpacked/other.x86_64", elf());
    let found = programs::find(dir.path(), true, LINUX);
    assert_eq!(names(dir.path(), &found), ["game.x86_64"]);
}

#[test]
fn what_is_a_native_program_is_decided_by_its_first_bytes() {
    let dir = tempfile::tempdir().expect("dir");
    assert!(programs::is_native_executable(&put(dir.path(), "elf", elf())));
    assert!(programs::is_native_executable(&put(dir.path(), "script", "#!/usr/bin/env bash\n")));
    assert!(programs::is_native_executable(&put(dir.path(), "script2", "#! /bin/sh\n")));
    assert!(!programs::is_native_executable(&put(dir.path(), "relative-shebang", "#!bash\n")));
    assert!(!programs::is_native_executable(&put(dir.path(), "text", "just words")));
    assert!(!programs::is_native_executable(&put(dir.path(), "empty", "")));
    let mut library = elf();
    library[24..32].fill(0); // no entry point: a shared library
    assert!(!programs::is_native_executable(&put(dir.path(), "lib", library)));
    let mut object = elf();
    object[16] = 1; // a relocatable object file
    assert!(!programs::is_native_executable(&put(dir.path(), "obj", object)));
    assert!(!programs::is_native_executable(&dir.path().join("missing")));
}

#[cfg(unix)]
#[test]
fn programs_are_made_executable_and_nothing_else_is() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("dir");
    let program = put(dir.path(), "Game", elf());
    let data = put(dir.path(), "data.bin", "not a program");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o644)).expect("mode");
    programs::make_runnable(dir.path(), LINUX);
    let mode = |p: &Path| fs::metadata(p).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode(&program), 0o755);
    assert_eq!(mode(&data) & 0o111, 0);
}

#[test]
fn marker_files_are_created_once_and_never_overwritten() {
    let dir = tempfile::tempdir().expect("dir");
    fs::write(dir.path().join("keep.cfg"), "mine").expect("write");
    let names = vec!["portable.txt".to_string(), "keep.cfg".to_string(), "../out.txt".to_string(), " ".to_string()];
    assert_eq!(add_marker_files(dir.path(), &names).expect("adds"), vec!["portable.txt".to_string()]);
    assert_eq!(fs::read_to_string(dir.path().join("keep.cfg")).expect("read"), "mine");
    assert!(!dir.path().parent().expect("parent").join("out.txt").exists());
    assert!(add_marker_files(dir.path(), &names).expect("again").is_empty(), "already there");
}
