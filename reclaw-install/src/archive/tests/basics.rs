use std::fs;

use reclaw_net::Cancel;

use super::*;

fn check_game(run: &Run, links: bool) {
    let game = run.dest.join("Game");
    assert_eq!(fs::read(game.join("game.x86_64")).expect("program"), elf());
    assert_eq!(fs::read_to_string(game.join("data/readme.txt")).expect("readme"), "hello");
    assert_eq!(fs::read(game.join("lib/libfoo.so.1")).expect("lib"), b"lib");
    #[cfg(unix)]
    if links {
        assert_eq!(fs::read_link(game.join("lib/libfoo.so")).expect("a link"), std::path::Path::new("libfoo.so.1"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = |p: &str| fs::metadata(game.join(p)).expect("meta").permissions().mode() & 0o777;
        if links {
            assert_eq!(mode("game.x86_64"), 0o755, "the execute bit survives");
        }
        assert_eq!(mode("data/readme.txt") & 0o111, 0, "a data file is not executable");
    }
}

#[test]
fn a_zip_unpacks_with_its_modes_and_links() {
    let (_keep, path) = write_archive("game.zip", &zip_bytes(&game_items()));
    let (run, result, positions) = run_in(Format::Zip, &path, Limits::default(), &Cancel::new());
    let stats = result.expect("unpacked");
    assert_eq!(stats.bytes, (elf().len() + 5 + 3) as u64);
    check_game(&run, true);
    assert!(!positions.is_empty() && positions.iter().all(|p| p.total.is_some()), "a zip knows its size in advance: {positions:?}");
}

#[test]
fn a_tar_gz_unpacks_with_its_modes_and_links() {
    let (_keep, path) = write_archive("game.tar.gz", &targz_bytes(&game_items()));
    let (run, result, _) = run_in(Format::TarGz, &path, Limits::default(), &Cancel::new());
    result.expect("unpacked");
    check_game(&run, true);
}

#[test]
fn a_tar_xz_unpacks() {
    let (_keep, path) = write_archive("game.tar.xz", &tarxz_bytes(&game_items()));
    let (run, result, _) = run_in(Format::TarXz, &path, Limits::default(), &Cancel::new());
    result.expect("unpacked");
    check_game(&run, true);
    assert!(!run.dir.path().join("payload.tar").exists(), "the intermediate tar does not stay");
}

#[test]
fn a_7z_unpacks() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = sevenz_file(dir.path(), &game_items());
    let (run, result, _) = run_in(Format::SevenZip, &path, Limits::default(), &Cancel::new());
    result.expect("unpacked");
    // 7z built from a folder keeps no Unix modes or links in this writer; the files are what matter here.
    check_game(&run, false);
}

#[test]
fn a_file_that_is_not_an_archive_is_said_not_to_be() {
    for (format, name) in [(Format::Zip, "x.zip"), (Format::TarGz, "x.tar.gz"), (Format::SevenZip, "x.7z"), (Format::TarXz, "x.tar.xz")] {
        let (_keep, path) = write_archive(name, b"this is certainly not an archive of any kind");
        let (_run, result, _) = run_in(format, &path, Limits::default(), &Cancel::new());
        assert!(matches!(result, Err(InstallError::Archive(_))), "{format:?}: {result:?}");
    }
}

#[test]
fn a_missing_file_is_an_io_error_naming_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (_run, result, _) = run_in(Format::Zip, &dir.path().join("nope.zip"), Limits::default(), &Cancel::new());
    assert!(matches!(&result, Err(InstallError::Io { what, .. }) if what.contains("nope.zip")), "{result:?}");
}

#[test]
fn windows_style_names_become_folders() {
    let (_keep, path) = write_archive("win.zip", &zip_bytes(&[Item::File("Game\\data\\a.txt", b"x".to_vec(), 0o644)]));
    let (run, result, _) = run_in(Format::Zip, &path, Limits::default(), &Cancel::new());
    result.expect("unpacked");
    assert_eq!(fs::read_to_string(run.dest.join("Game/data/a.txt")).expect("file"), "x");
}

#[test]
fn a_cancelled_extraction_stops_with_cancelled() {
    let (_keep, path) = write_archive("game.zip", &zip_bytes(&game_items()));
    let cancel = Cancel::new();
    cancel.cancel();
    let (_run, result, _) = run_in(Format::Zip, &path, Limits::default(), &cancel);
    assert_eq!(result, Err(InstallError::Cancelled));
}

#[test]
fn a_rar_with_no_tool_to_read_it_says_what_to_install() {
    let (_keep, path) = write_archive("game.rar", b"Rar!\x1a\x07\x00 not really");
    // Whatever tools this machine has, a file like this cannot be unpacked.
    let (_run, result, _) = run_in(Format::Rar, &path, Limits::default(), &Cancel::new());
    assert!(matches!(result, Err(InstallError::Unsupported(_))), "{result:?}");
}
