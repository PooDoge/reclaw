use std::{fs::File, io::BufReader, path::Path};

use super::sink::Sink;
use crate::error::InstallError;

/// Unpack a zip. Entries are read one after another from the central directory; the total is known from it, so progress is exact.
pub fn extract(archive: &Path, sink: &mut Sink<'_>) -> Result<(), InstallError> {
    let file = File::open(archive).map_err(|e| InstallError::io(format!("opening {}", archive.display()), &e))?;
    let mut zip =
        zip::ZipArchive::new(BufReader::new(file)).map_err(|e| InstallError::Archive(format!("this is not a readable zip file: {e}")))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| InstallError::Archive(format!("a zip entry could not be read: {e}")))?;
        let name = entry.name().to_string();
        let mode = entry.unix_mode();
        if entry.is_dir() || name.ends_with('/') {
            sink.dir(&name, mode)?;
        } else if entry.is_symlink() {
            // A link's content is its target.
            let mut target = String::new();
            std::io::Read::take(&mut entry, 4096)
                .read_to_string(&mut target)
                .map_err(|e| InstallError::Archive(format!("the link {name} could not be read: {e}")))?;
            sink.symlink(&name, &target)?;
        } else {
            sink.file(&name, mode, &mut entry)?;
        }
    }
    Ok(())
}

use std::io::Read as _;

/// The unpacked size of a zip, from its directory, for a progress bar that fills.
pub fn unpacked_size(archive: &Path) -> Option<u64> {
    let file = File::open(archive).ok()?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file)).ok()?;
    let mut total = 0u64;
    for index in 0..zip.len() {
        total = total.saturating_add(zip.by_index_raw(index).ok()?.size());
    }
    Some(total)
}
