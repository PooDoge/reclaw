use std::fs;

use reclaw_net::Cancel;

use super::*;

fn unpack_tar(items: &[Item]) -> (Run, Result<Stats, InstallError>) {
    let (_keep, path) = write_archive("evil.tar.gz", &targz_bytes(items));
    let (run, result, _) = run_in(Format::TarGz, &path, Limits::default(), &Cancel::new());
    (run, result)
}

fn outside(run: &Run, name: &str) -> bool {
    run.dir.path().join(name).exists()
}

#[test]
fn an_entry_that_climbs_out_is_refused_and_nothing_is_written_outside() {
    for name in ["../evil.txt", "a/../../evil.txt", "..\\evil.txt", "/evil.txt", "a/b/../../../evil.txt"] {
        let (run, result) = unpack_tar(&[Item::File(Box::leak(name.to_string().into_boxed_str()), b"x".to_vec(), 0o644)]);
        // Absolute and climbing names are refused; a leading slash alone is cut by the tar library into a relative one.
        if let Err(error) = &result {
            assert!(matches!(error, InstallError::Archive(m) if m.contains("leave its folder")), "{name}: {error:?}");
        }
        assert!(!outside(&run, "evil.txt"), "{name}: written outside");
        assert!(!run.dest.join("../../evil.txt").exists());
    }
}

#[test]
fn the_same_in_a_zip() {
    for name in ["../evil.txt", "a/../../evil.txt", "..\\evil.txt"] {
        let (_keep, path) =
            write_archive("evil.zip", &zip_bytes(&[Item::File(Box::leak(name.to_string().into_boxed_str()), b"x".to_vec(), 0o644)]));
        let (run, result, _) = run_in(Format::Zip, &path, Limits::default(), &Cancel::new());
        assert!(matches!(&result, Err(InstallError::Archive(m)) if m.contains("leave its folder")), "{name}: {result:?}");
        assert!(!outside(&run, "evil.txt"), "{name}");
    }
}

#[cfg(unix)]
#[test]
fn a_link_pointing_outside_is_left_out_but_the_rest_is_unpacked() {
    let (run, result) = unpack_tar(&[
        Item::Link("escape", "../../etc"),
        Item::Link("absolute", "/etc/passwd"),
        Item::File("fine.txt", b"ok".to_vec(), 0o644),
    ]);
    let stats = result.expect("the install goes on");
    assert_eq!(stats.skipped, 2);
    assert!(fs::symlink_metadata(run.dest.join("escape")).is_err());
    assert!(fs::symlink_metadata(run.dest.join("absolute")).is_err());
    assert_eq!(fs::read_to_string(run.dest.join("fine.txt")).expect("file"), "ok");
}

#[cfg(unix)]
#[test]
fn nothing_is_written_through_a_link_an_earlier_entry_made() {
    // `d` is a perfectly good link to a folder inside; a file "in" it would land wherever the link goes.
    let (_run, result) = unpack_tar(&[Item::Dir("real/"), Item::Link("d", "real"), Item::File("d/x.txt", b"x".to_vec(), 0o644)]);
    assert!(matches!(&result, Err(InstallError::Archive(m)) if m.contains("through a link")), "{result:?}");
}

#[cfg(unix)]
#[test]
fn the_link_chain_trick_cannot_reach_out() {
    // d -> .. makes `a/d` the folder above `a`; a link in it that is "fine" lexically would then be wrong. The write is refused first.
    let (run, result) = unpack_tar(&[Item::Dir("a/"), Item::Link("a/d", ".."), Item::Link("a/d/l", "../..")]);
    assert!(matches!(&result, Err(InstallError::Archive(m)) if m.contains("through a link")), "{result:?}");
    assert!(!outside(&run, "l"));
}

#[test]
fn a_file_where_a_folder_is_needed_is_an_error() {
    let (_run, result) = unpack_tar(&[Item::File("a", b"x".to_vec(), 0o644), Item::File("a/b", b"y".to_vec(), 0o644)]);
    assert!(matches!(&result, Err(InstallError::Archive(m)) if m.contains("file where a folder is needed")), "{result:?}");
}

#[cfg(unix)]
#[test]
fn set_id_bits_are_dropped() {
    use std::os::unix::fs::PermissionsExt;
    let (run, result) = unpack_tar(&[Item::File("suid", elf(), 0o6755)]);
    result.expect("unpacked");
    let mode = fs::metadata(run.dest.join("suid")).expect("meta").permissions().mode();
    assert_eq!(mode & 0o7000, 0, "{mode:o}");
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn too_many_entries_is_a_bomb() {
    let items: Vec<Item> = (0..20).map(|i| Item::File(Box::leak(format!("f{i}").into_boxed_str()), b"x".to_vec(), 0o644)).collect();
    let (_keep, path) = write_archive("many.zip", &zip_bytes(&items));
    let (_run, result, _) = run_in(Format::Zip, &path, Limits { max_bytes: u64::MAX, max_entries: 10 }, &Cancel::new());
    assert!(matches!(&result, Err(InstallError::Archive(m)) if m.contains("more than 10 entries")), "{result:?}");
}

#[test]
fn too_many_bytes_is_a_bomb() {
    let (_keep, path) = write_archive("big.zip", &zip_bytes(&[Item::File("zeros", vec![0u8; 100_000], 0o644)]));
    let (_run, result, _) = run_in(Format::Zip, &path, Limits { max_bytes: 1000, max_entries: 100 }, &Cancel::new());
    assert!(matches!(&result, Err(InstallError::Archive(m)) if m.contains("likely bomb")), "{result:?}");
}

#[test]
fn a_dot_entry_for_the_archives_own_folder_is_harmless() {
    let (run, result) = unpack_tar(&[Item::Dir("./"), Item::File("./a.txt", b"x".to_vec(), 0o644)]);
    result.expect("unpacked");
    assert!(run.dest.join("a.txt").is_file());
}
