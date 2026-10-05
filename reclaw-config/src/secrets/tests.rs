use super::*;

fn file(dir: &tempfile::TempDir) -> SecretsFile {
    SecretsFile::at(dir.path().join("config").join("secrets.toml"))
}

fn tokens(github: Option<&str>, gitlab: Option<&str>) -> Secrets {
    let mut s = Secrets::default();
    s.set("github", github.map(Secret::new));
    s.set("gitlab", gitlab.map(Secret::new));
    s
}

#[test]
fn tokens_survive_a_save_and_a_load_and_the_directory_is_made() {
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    f.save(&tokens(Some("ghp_round_trip_0000001"), Some("glpat-round-trip-02"))).expect("save");
    let loaded = f.load();
    assert!(loaded.warning.is_none() && !loaded.read_only, "{loaded:?}");
    assert_eq!(loaded.secrets.get("github").map(Secret::expose), Some("ghp_round_trip_0000001"));
    assert_eq!(loaded.secrets.get("gitlab").map(Secret::expose), Some("glpat-round-trip-02"));
}

#[test]
fn no_file_is_no_tokens_and_no_warning() {
    let dir = tempfile::tempdir().expect("tempdir");
    let loaded = file(&dir).load();
    assert!(loaded.secrets.is_empty() && loaded.warning.is_none() && !loaded.read_only);
}

#[cfg(unix)]
#[test]
fn the_file_is_private_from_the_first_byte_and_a_looser_one_is_tightened_with_a_warning() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    f.save(&tokens(Some("ghp_private_file_000003"), None)).expect("save");
    let mode = |p: &Path| fs::metadata(p).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode(f.path()), 0o600);

    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o644)).expect("loosen");
    let loaded = f.load();
    assert!(loaded.warning.as_deref().is_some_and(|w| w.contains("644") && w.contains("private now")), "{:?}", loaded.warning);
    assert_eq!(mode(f.path()), 0o600, "tightened");
    assert_eq!(loaded.secrets.get("github").map(Secret::expose), Some("ghp_private_file_000003"), "still read");
    assert!(f.load().warning.is_none(), "told once");
}

#[test]
fn a_replacement_leaves_no_temporary_file_and_removing_the_last_token_removes_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    f.save(&tokens(Some("ghp_first_token_0000004"), None)).expect("save");
    f.save(&tokens(Some("ghp_second_token_000005"), None)).expect("replace");
    let names: Vec<String> = fs::read_dir(f.path().parent().expect("parent"))
        .expect("read")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["secrets.toml"], "{names:?}");
    assert_eq!(f.load().secrets.get("github").map(Secret::expose), Some("ghp_second_token_000005"));
    f.save(&Secrets::default()).expect("remove");
    assert!(!f.path().exists());
    f.save(&Secrets::default()).expect("removing what is not there is fine");
}

#[test]
fn a_damaged_file_is_set_aside_not_deleted_and_the_program_starts_without_tokens() {
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    fs::create_dir_all(f.path().parent().expect("parent")).expect("mkdir");
    fs::write(f.path(), "version = 1\n[tokens\ngithub = \"ghp_keep_me_00000006\"").expect("seed");
    let loaded = f.load();
    assert!(loaded.secrets.is_empty() && !loaded.read_only);
    assert!(loaded.warning.as_deref().is_some_and(|w| w.contains("secrets.toml.bad")), "{:?}", loaded.warning);
    let kept = fs::read_to_string(f.path().with_extension("toml.bad")).expect("the damaged file is kept");
    assert!(kept.contains("ghp_keep_me_00000006"), "so the person can recover the token by hand");
}

#[test]
fn a_file_from_a_newer_reclaw_is_not_used_and_not_overwritten() {
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    fs::create_dir_all(f.path().parent().expect("parent")).expect("mkdir");
    fs::write(f.path(), "version = 9\n[tokens]\ngithub = \"ghp_future_000000007\"\n").expect("seed");
    let loaded = f.load();
    assert!(loaded.secrets.is_empty() && loaded.read_only);
    assert!(loaded.warning.as_deref().is_some_and(|w| w.contains("newer Reclaw")));
}

#[test]
fn a_file_that_cannot_be_read_is_read_only_and_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    // A directory where the file should be: reading it fails with something other than "not found".
    fs::create_dir_all(f.path()).expect("mkdir");
    let loaded = f.load();
    assert!(loaded.read_only && loaded.warning.is_some(), "{loaded:?}");
    assert!(f.path().is_dir());
}

#[test]
fn blank_tokens_are_ignored_and_whitespace_is_trimmed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    fs::create_dir_all(f.path().parent().expect("parent")).expect("mkdir");
    fs::write(f.path(), "version = 1\n[tokens]\ngithub = \"  ghp_padded_00000008  \"\ngitlab = \"   \"\n").expect("seed");
    let loaded = f.load();
    assert_eq!(loaded.secrets.get("github").map(Secret::expose), Some("ghp_padded_00000008"));
    assert!(loaded.secrets.get("gitlab").is_none());
}

#[test]
fn nothing_printed_shows_a_token_and_loading_registers_it_for_the_log() {
    let dir = tempfile::tempdir().expect("tempdir");
    let f = file(&dir);
    f.save(&tokens(Some("unusual-company-token-shape-9"), None)).expect("save");
    let loaded = f.load();
    assert!(!format!("{loaded:?}").contains("unusual-company"), "{loaded:?}");
    assert_eq!(reclaw_log::scrub("failed with unusual-company-token-shape-9 here"), format!("failed with {} here", reclaw_log::REDACTED));
    let err = SecretsError::Io { path: f.path().to_path_buf(), source: std::io::Error::other("disk") };
    assert!(!err.to_string().contains("unusual-company"));
}
