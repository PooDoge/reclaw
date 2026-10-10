use super::*;

fn first_install_left_nothing_behind(rig: &Rig) {
    let folder = rig.folder();
    assert!(!folder.join(VERSION_FILE).exists(), "no version means not installed");
    assert!(!folder.join(STAGE_DIR).exists(), "the staging folder is cleaned up");
    assert_eq!(layout::installed_version(&folder), None);
}

#[test]
fn a_file_that_is_not_what_the_release_promised_is_refused_and_not_kept() {
    let mut file = Served::new("Game-Linux.zip", game_zip("1")).with_digest();
    file.digest = Some(format!("sha256:{}", "0".repeat(64)));
    let rig = Rig::new(vec![("v1", false, vec![file])]);
    let error = rig.install(&rig.request()).expect_err("refused");
    assert!(matches!(error, InstallError::Net(NetError::Integrity(_))), "{error:?}");
    first_install_left_nothing_behind(&rig);
}

#[test]
fn a_cancelled_install_stops_and_keeps_nothing_installed() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", game_zip("1"))])]);
    let request = rig.request();
    let resolved = rig.ready(&request);
    let cancel = Cancel::new();
    cancel.cancel();
    let error = rig.installer.install(&request, &resolved, &cancel, &mut |_| {}).expect_err("cancelled");
    assert!(error.is_cancelled(), "{error:?}");
    first_install_left_nothing_behind(&rig);
}

#[test]
fn a_corrupt_archive_fails_clearly_and_the_next_try_works() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", b"PK this is not really a zip".to_vec())])]);
    let request = rig.request();
    let error = rig.install(&request).expect_err("corrupt");
    assert!(matches!(&error, InstallError::Archive(m) if m.contains("not a readable zip")), "{error:?}");
    first_install_left_nothing_behind(&rig);
    assert!(rig.folder().join(INCOMPLETE_FILE).exists(), "a first install that failed says so");

    *rig.releases.lock().expect("lock") = vec![("v1", false, vec![Served::new("Game-Linux.zip", game_zip("1"))])];
    rig.install(&request).expect("the retry works");
    assert!(!rig.folder().join(INCOMPLETE_FILE).exists());
    assert_eq!(read(rig.folder().join(VERSION_FILE)), "v1");
}

#[test]
fn a_download_with_nothing_to_start_is_an_error_not_an_install() {
    let archive = zip_bytes(&[Item::File("README.md", b"only words".to_vec(), 0o644)]);
    let rig = Rig::new(vec![(
        "v1",
        false,
        vec![
            Served::new("Game-Linux-x64.tar.gz", targz_bytes(&[Item::File("notes.txt", b"x".to_vec(), 0o644)])),
            Served::new("Other-Windows.zip", archive),
        ],
    )]);
    let mut request = rig.request();
    request.asset = Some("Game-Linux-x64.tar.gz".to_string());
    let error = rig.install(&request).expect_err("nothing to start");
    assert_eq!(error, InstallError::NoProgram);
    assert!(rig.folder().join(INCOMPLETE_FILE).exists());
    assert!(!rig.folder().join(VERSION_FILE).exists());
}

#[test]
fn a_failed_update_leaves_the_working_install_working() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", game_zip("1"))])]);
    let request = rig.request();
    rig.install(&request).expect("first install");
    *rig.releases.lock().expect("lock") = vec![("v2", false, vec![Served::new("Game-Linux.zip", b"PK broken".to_vec())])];
    rig.install(&request).expect_err("broken update");
    let folder = rig.folder();
    assert_eq!(read(folder.join(VERSION_FILE)), "v1", "still the old version");
    assert!(!folder.join(INCOMPLETE_FILE).exists(), "an update never marks a working install incomplete");
    assert!(layout::is_complete(&folder, Platform::LinuxX64));
}

#[test]
fn a_release_with_nothing_for_this_system_says_why() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-macos.zip", vec![1]), Served::new("Game.sha256", vec![2])])]);
    let error = rig.installer.resolve(&rig.request()).expect_err("nothing fits");
    assert_eq!(error, InstallError::NoDownload("This release has no recognized download for Linux-X64.".to_string()));
}

#[test]
fn a_package_for_the_system_installer_is_named_not_attempted() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-linux.deb", vec![1])])]);
    let error = rig.installer.resolve(&rig.request()).expect_err("unsupported");
    assert!(matches!(&error, InstallError::NoDownload(m) if m.contains("system package")), "{error:?}");
}

#[test]
fn a_release_page_that_is_gone_is_no_releases() {
    let rig = Rig::new(vec![]);
    assert_eq!(rig.installer.resolve(&rig.request()), Err(InstallError::NoReleases { repo: "o/r".to_string() }));
}

#[test]
fn the_folder_not_being_writable_is_an_io_error_naming_it() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", game_zip("1"))])]);
    let mut request = rig.request();
    // A file where the folder's parent should be.
    fs::write(rig.root.path().join("blocker"), "x").expect("blocker");
    request.folder = rig.root.path().join("blocker/Game");
    let error = rig.install(&request).expect_err("cannot make the folder");
    assert!(matches!(&error, InstallError::Io { what, .. } if what.contains("blocker")), "{error:?}");
    assert!(error.hint().is_some());
}

#[test]
fn a_hostile_archive_cannot_write_outside_the_app_folder() {
    let archive = zip_bytes(&[Item::File("../escaped.txt", b"x".to_vec(), 0o644), Item::File("Game", elf(), 0o755)]);
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", archive)])]);
    let error = rig.install(&rig.request()).expect_err("refused");
    assert!(matches!(&error, InstallError::Archive(m) if m.contains("leave its folder")), "{error:?}");
    assert!(!rig.folder().join("../escaped.txt").exists() && !rig.folder().join(STAGE_DIR).join("escaped.txt").exists());
    first_install_left_nothing_behind(&rig);
}

#[test]
fn releases_given_with_the_request_are_used_without_asking_the_host_and_their_checksums_hold() {
    let body = game_zip("1");
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", body.clone())])]);
    let known = |sha256: String| Release {
        tag: "v1".into(),
        assets: vec![Asset {
            name: "Game-Linux.zip".into(),
            url: rig.server.url("/dl/v1/Game-Linux.zip"),
            size: None,
            sha256: Some(sha256),
        }],
        ..Default::default()
    };
    let request = Request { releases: Some(vec![known("0".repeat(64))]), ..rig.request() };
    let error = rig.install(&request).expect_err("refused");
    assert!(matches!(error, InstallError::Net(NetError::Integrity(_))), "the listed checksum is checked: {error:?}");
    let request = Request { releases: Some(vec![known(sha256_hex(&body))]), ..rig.request() };
    assert_eq!(rig.install(&request).expect("installed").version, "v1");
    assert!(rig.server.requests().iter().all(|r| !r.path.starts_with("/repos/")), "the host was never asked");
}
