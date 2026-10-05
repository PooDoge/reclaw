use std::{fs::File, io::BufReader, path::Path};

use sevenz_rust2::{ArchiveReader, Password};

use super::sink::Sink;
use crate::error::InstallError;

/// 7-Zip keeps a Unix mode in the high half of the attributes when bit 15 says so.
fn unix_mode(attributes: u32) -> Option<u32> {
    (attributes & 0x8000 != 0).then_some(attributes >> 16)
}

/// Unpack a 7z (LZMA, LZMA2, BZip2, Deflate...). An encrypted archive is refused: Reclaw never asks for a password.
pub fn extract(archive: &Path, sink: &mut Sink<'_>) -> Result<(), InstallError> {
    let file = File::open(archive).map_err(|e| InstallError::io(format!("opening {}", archive.display()), &e))?;
    let mut reader = ArchiveReader::new(BufReader::new(file), Password::empty())
        .map_err(|e| InstallError::Archive(format!("this is not a readable 7z file (or it needs a password): {e}")))?;
    let mut failure: Option<InstallError> = None;
    let result = reader.for_each_entries(|entry, data| {
        let name = entry.name().to_string();
        let mode = unix_mode(entry.windows_attributes());
        let step = if entry.is_directory() {
            sink.dir(&name, mode)
        } else if mode.is_some_and(|m| m & 0o170000 == 0o120000) {
            let mut target = String::new();
            std::io::Read::read_to_string(&mut std::io::Read::take(data, 4096), &mut target)
                .map_err(|e| InstallError::Archive(format!("the link {name} could not be read: {e}")))
                .and_then(|_| sink.symlink(&name, &target))
        } else if entry.is_anti_item() {
            // A deletion marker from an incremental archive: nothing to unpack.
            Ok(())
        } else {
            // An entry with no stream is an empty file; the reader then yields no bytes.
            sink.file(&name, mode.map(|m| m & 0o7777), data)
        };
        match step {
            Ok(()) => Ok(true),
            Err(error) => {
                // The reader's error type is its own; keep ours aside and stop the walk.
                failure = Some(error);
                Ok(false)
            }
        }
    });
    if let Some(error) = failure {
        return Err(error);
    }
    result.map_err(|e| InstallError::Archive(format!("the 7z file could not be unpacked: {e}")))
}

/// The unpacked size, from the archive's index.
pub fn unpacked_size(archive: &Path) -> Option<u64> {
    let file = File::open(archive).ok()?;
    let reader = ArchiveReader::new(BufReader::new(file), Password::empty()).ok()?;
    Some(reader.archive().files.iter().map(|f| f.size()).sum())
}
