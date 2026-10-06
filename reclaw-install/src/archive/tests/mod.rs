//! Archives built in the tests (real ones, in every format), unpacked into a temporary folder.
mod basics;
mod hostile;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use reclaw_net::Cancel;

use super::{Limits, Position, Stats, extract};
use crate::{error::InstallError, format::Format};

/// An ELF program header good enough for the discovery rules: 64-bit, little-endian, executable, with an entry point.
pub fn elf() -> Vec<u8> {
    let mut header = vec![0u8; 64];
    header[..4].copy_from_slice(b"\x7fELF");
    header[4] = 2;
    header[5] = 1;
    header[6] = 1;
    header[16] = 2;
    header[24] = 0x10;
    header[25] = 0x40;
    header
}

/// One entry of an archive being built.
pub enum Item {
    Dir(&'static str),
    File(&'static str, Vec<u8>, u32),
    Link(&'static str, &'static str),
}

pub fn zip_bytes(items: &[Item]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for item in items {
        match item {
            Item::Dir(name) => writer.add_directory(*name, zip::write::SimpleFileOptions::default()).expect("dir"),
            Item::File(name, data, mode) => {
                let options =
                    zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(*mode);
                writer.start_file(*name, options).expect("file");
                writer.write_all(data).expect("data");
            }
            Item::Link(name, target) => writer.add_symlink(*name, *target, zip::write::SimpleFileOptions::default()).expect("link"),
        }
    }
    writer.finish().expect("finish").into_inner()
}

/// A tar. `raw_names` skips the library's refusal of `..`, so a hostile archive can be built.
pub fn tar_bytes(items: &[Item]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for item in items {
        let mut header = tar::Header::new_gnu();
        let (name, kind, data, mode, link): (&str, tar::EntryType, &[u8], u32, Option<&str>) = match item {
            Item::Dir(name) => (name, tar::EntryType::Directory, &[], 0o755, None),
            Item::File(name, data, mode) => (name, tar::EntryType::Regular, data, *mode, None),
            Item::Link(name, target) => (name, tar::EntryType::Symlink, &[], 0o777, Some(target)),
        };
        header.set_entry_type(kind);
        header.set_mode(mode);
        header.set_size(data.len() as u64);
        let field = &mut header.as_old_mut().name;
        field.fill(0);
        field[..name.len()].copy_from_slice(name.as_bytes());
        if let Some(target) = link {
            header.as_old_mut().linkname[..target.len()].copy_from_slice(target.as_bytes());
        }
        header.set_cksum();
        builder.append(&header, data).expect("append");
    }
    builder.into_inner().expect("tar")
}

pub fn targz_bytes(items: &[Item]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&tar_bytes(items)).expect("gz");
    encoder.finish().expect("gz finish")
}

pub fn tarxz_bytes(items: &[Item]) -> Vec<u8> {
    let tar = tar_bytes(items);
    let mut out = Vec::new();
    lzma_rs::xz_compress(&mut std::io::Cursor::new(tar), &mut out).expect("xz");
    out
}

/// A 7z made from a folder of the same files.
pub fn sevenz_file(dir: &Path, items: &[Item]) -> PathBuf {
    let source = dir.join("to-pack");
    for item in items {
        match item {
            Item::Dir(name) => fs::create_dir_all(source.join(name)).expect("dir"),
            Item::File(name, data, _) => {
                let path = source.join(name);
                fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
                fs::write(path, data).expect("file");
            }
            Item::Link(..) => {}
        }
    }
    let out = dir.join("pack.7z");
    sevenz_rust2::compress_to_path(&source, &out).expect("7z");
    out
}

/// A folder tree the standard contents of a small game, for every format.
pub fn game_items() -> Vec<Item> {
    vec![
        Item::Dir("Game/"),
        Item::File("Game/game.x86_64", elf(), 0o755),
        Item::File("Game/data/readme.txt", b"hello".to_vec(), 0o644),
        Item::File("Game/lib/libfoo.so.1", b"lib".to_vec(), 0o644),
        Item::Link("Game/lib/libfoo.so", "libfoo.so.1"),
    ]
}

pub struct Run {
    pub dir: tempfile::TempDir,
    pub dest: PathBuf,
}

pub fn run_in(format: Format, archive: &Path, limits: Limits, cancel: &Cancel) -> (Run, Result<Stats, InstallError>, Vec<Position>) {
    let dir = tempfile::tempdir().expect("temp dir");
    let dest = dir.path().join("out");
    fs::create_dir_all(&dest).expect("dest");
    let mut positions = Vec::new();
    let result = extract(format, archive, &dest, dir.path(), limits, cancel, &mut |p| positions.push(p));
    (Run { dir, dest }, result, positions)
}

pub fn write_archive(name: &str, bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("write archive");
    (dir, path)
}
