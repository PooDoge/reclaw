use super::*;

#[test]
fn a_zip_is_downloaded_unpacked_hoisted_and_versioned() {
    let rig = Rig::new(vec![(
        "v1.0.0",
        false,
        vec![Served::new("Game-Linux.zip", game_zip("1")).with_digest(), Served::new("Game-source.zip", vec![1])],
    )]);
    let request = rig.request();
    let resolved = rig.ready(&request);
    assert_eq!(
        (resolved.release.tag.as_str(), resolved.asset.name.as_str(), resolved.already_installed),
        ("v1.0.0", "Game-Linux.zip", false)
    );

    let mut steps = Vec::new();
    let done = rig.installer.install(&request, &resolved, &Cancel::new(), &mut |s| steps.push(s)).expect("installed");

    let folder = rig.folder();
    assert!(folder.join("game.x86_64").is_file(), "the wrapper folder was hoisted");
    assert_eq!(read(folder.join("data/readme.txt")), "hello");
    assert_eq!(read(folder.join(VERSION_FILE)), "v1.0.0");
    assert!(!folder.join(INCOMPLETE_FILE).exists() && !folder.join(STAGE_DIR).exists(), "nothing is left half-done");
    assert_eq!(done.version, "v1.0.0");
    assert_eq!(done.asset, "Game-Linux.zip");
    assert_eq!(done.program.as_deref(), Some(folder.join("game.x86_64").as_path()));
    assert!(!done.needs_runner);
    assert_eq!(done.notes, "Notes for v1.0.0");
    assert!(
        !rig.root.path().join("downloads").read_dir().expect("downloads").any(|e| e
            .expect("entry")
            .path()
            .read_dir()
            .is_ok_and(|mut d| d.next().is_some())),
        "the download was cleared"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(folder.join("game.x86_64")).expect("meta").permissions().mode() & 0o111,
            0o111,
            "a zip that lost its modes still runs"
        );
    }
    let stages: Vec<_> = steps.iter().map(|s| s.stage).collect();
    assert!(
        stages.contains(&Stage::Downloading) && stages.contains(&Stage::Extracting) && stages.last() == Some(&Stage::Finishing),
        "{stages:?}"
    );
    assert!(steps.iter().any(|s| s.stage == Stage::Downloading && s.total == Some(resolved.asset.size.expect("size"))), "{steps:?}");
}

#[test]
fn a_tar_gz_with_links_installs_like_a_zip() {
    let archive = targz_bytes(&[
        Item::Dir("Game/"),
        Item::File("Game/Game", elf(), 0o755),
        Item::File("Game/lib/libfoo.so.1", b"lib".to_vec(), 0o644),
        Item::Link("Game/lib/libfoo.so", "libfoo.so.1"),
    ]);
    let rig = Rig::new(vec![("v2", false, vec![Served::new("Game-linux-x64.tar.gz", archive)])]);
    let done = rig.install(&rig.request()).expect("installed");
    assert_eq!(done.program.as_deref(), Some(rig.folder().join("Game").as_path()));
    #[cfg(unix)]
    assert_eq!(fs::read_link(rig.folder().join("lib/libfoo.so")).expect("link"), Path::new("libfoo.so.1"));
}

#[test]
fn an_update_lays_new_files_over_old_ones_and_leaves_the_users_alone() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", game_zip("1"))])]);
    let request = rig.request();
    rig.install(&request).expect("first install");
    let folder = rig.folder();
    fs::create_dir_all(folder.join("saves")).expect("saves");
    fs::write(folder.join("saves/slot1.sav"), "my progress").expect("save");
    fs::write(folder.join("config.ini"), "my settings").expect("config");
    fs::write(folder.join("data/readme.txt"), "edited by me").expect("edit");

    *rig.releases.lock().expect("lock") = vec![
        ("v2", false, vec![Served::new("Game-Linux.zip", game_zip("2"))]),
        ("v1", false, vec![Served::new("Game-Linux.zip", game_zip("1"))]),
    ];
    let resolved = rig.ready(&request);
    assert_eq!(resolved.release.tag, "v2");
    assert!(!resolved.already_installed);
    rig.installer.install(&request, &resolved, &Cancel::new(), &mut |_| {}).expect("update");

    assert!(fs::read(folder.join("game.x86_64")).expect("program").ends_with(b"2"), "the program is the new one");
    assert_eq!(read(folder.join("saves/slot1.sav")), "my progress");
    assert_eq!(read(folder.join("config.ini")), "my settings");
    assert_eq!(read(folder.join("data/readme.txt")), "hello", "a file the release ships is replaced");
    assert_eq!(read(folder.join(VERSION_FILE)), "v2");
}

#[test]
fn an_install_of_the_current_release_is_recognised_before_anything_is_downloaded() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", game_zip("1"))])]);
    let request = rig.request();
    rig.install(&request).expect("installed");
    let asked = rig.server.count();
    let resolved = rig.ready(&request);
    assert!(resolved.already_installed);
    assert_eq!(rig.server.count(), asked + 1, "one question about releases, no download");
}

#[test]
fn a_single_appimage_is_copied_made_executable_and_replaces_the_old_one() {
    let image = [elf(), b"image 1".to_vec()].concat();
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-1.0-x86_64.AppImage", image)])]);
    let request = rig.request();
    let first = rig.install(&request).expect("installed");
    let folder = rig.folder();
    assert_eq!(first.program.as_deref(), Some(folder.join("Game-1.0-x86_64.AppImage").as_path()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(folder.join("Game-1.0-x86_64.AppImage")).expect("meta").permissions().mode() & 0o111, 0o111);
    }

    *rig.releases.lock().expect("lock") =
        vec![("v2", false, vec![Served::new("Game-2.0-x86_64.AppImage", [elf(), b"image 2".to_vec()].concat())])];
    rig.install(&request).expect("updated");
    assert!(folder.join("Game-2.0-x86_64.AppImage").is_file());
    assert!(!folder.join("Game-1.0-x86_64.AppImage").exists(), "the old image is gone");
}

#[test]
fn a_windows_build_alone_installs_and_says_it_needs_a_runner() {
    let archive = zip_bytes(&[Item::File("Game.exe", b"MZ".to_vec(), 0o644), Item::File("data.pak", b"x".to_vec(), 0o644)]);
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Windows.zip", archive)])]);
    let done = rig.install(&rig.request()).expect("installed");
    assert!(done.needs_runner);
    assert_eq!(done.program.as_deref(), Some(rig.folder().join("Game.exe").as_path()));
}

#[test]
fn two_native_downloads_ask_and_the_answer_installs() {
    let rig = Rig::new(vec![(
        "v1",
        false,
        vec![
            Served::new("Game-linux-x64.tar.gz", targz_bytes(&[Item::File("Game", elf(), 0o755)])),
            Served::new("Game-x86_64.AppImage", elf()),
        ],
    )]);
    let mut request = rig.request();
    let Plan::Choose { choices, release } = rig.plan(&request) else { panic!("expected a question") };
    assert_eq!((release.tag.as_str(), choices.len()), ("v1", 2));
    request.asset = Some("Game-x86_64.AppImage".to_string());
    let done = rig.install(&request).expect("installed");
    assert_eq!(done.asset, "Game-x86_64.AppImage");
}

#[test]
fn the_catalogs_filter_settles_it_without_asking() {
    let rig = Rig::new(vec![(
        "v1",
        false,
        vec![
            Served::new("Game-linux-x64.tar.gz", targz_bytes(&[Item::File("Game", elf(), 0o755)])),
            Served::new("Game-x86_64.AppImage", elf()),
        ],
    )]);
    let mut request = rig.request();
    request.filter = Some("appimage".to_string());
    assert_eq!(rig.ready(&request).asset.name, "Game-x86_64.AppImage");
}

#[test]
fn a_prerelease_is_installed_only_when_asked_for() {
    let rig = Rig::new(vec![
        ("v3-beta", true, vec![Served::new("Game-Linux.zip", game_zip("beta"))]),
        ("v2", false, vec![Served::new("Game-Linux.zip", game_zip("2"))]),
    ]);
    let mut request = rig.request();
    assert_eq!(rig.ready(&request).release.tag, "v2");
    request.allow_prerelease = true;
    assert_eq!(rig.ready(&request).release.tag, "v3-beta");
    request.allow_prerelease = false;
    request.preferred_version = Some("3-beta".to_string());
    assert_eq!(rig.ready(&request).release.tag, "v3-beta", "a pin is honoured, whatever it is");
}

#[test]
fn an_archive_with_no_name_is_recognised_by_its_first_bytes() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux", game_zip("1"))])]);
    // "Game-Linux" has no extension: a name like a GitLab package link. It is a zip.
    let done = rig.install(&rig.request()).expect("installed");
    assert!(rig.folder().join("game.x86_64").is_file());
    assert_eq!(done.asset, "Game-Linux");
}

#[test]
fn a_bare_program_is_just_a_program() {
    let rig = Rig::new(vec![("v1", false, vec![Served::new("CrashBandicoot_Linux", elf())])]);
    let done = rig.install(&rig.request()).expect("installed");
    assert_eq!(done.program.as_deref(), Some(rig.folder().join("CrashBandicoot_Linux").as_path()));
}

#[test]
fn a_zip_inside_a_zip_that_holds_only_a_tarball_is_unwrapped() {
    let inner = targz_bytes(&[Item::File("Game", elf(), 0o755), Item::File("data/a.txt", b"a".to_vec(), 0o644)]);
    let outer = zip_bytes(&[Item::File("Game-linux.tar.gz", inner, 0o644)]);
    let rig = Rig::new(vec![("v1", false, vec![Served::new("Game-Linux.zip", outer)])]);
    rig.install(&rig.request()).expect("installed");
    assert!(rig.folder().join("Game").is_file() && rig.folder().join("data/a.txt").is_file());
    assert!(!rig.folder().join("Game-linux.tar.gz").exists());
}
