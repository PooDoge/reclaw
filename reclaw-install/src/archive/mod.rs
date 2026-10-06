//! Unpacking a downloaded archive into a folder, safely. Zip, tar.gz, tar.xz and 7z are read by this program (no system
//! library, no tool); RAR, and a zip using a method the reader lacks, go to a tool on the machine if there is one.
//!
//! * `sink`: where entries are written, and the rules that keep an archive inside its folder (all formats go through it)
//! * `zip_format`, `tar_format`, `sevenz_format`: one reader each; `external`: the system tools
use std::path::Path;

use reclaw_net::Cancel;

use crate::{error::InstallError, format::Format};

mod external;
mod sevenz_format;
mod sink;
mod tar_format;
mod zip_format;

pub use sink::{Limits, Position, safe_relative};

/// What an extraction did.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Stats {
    pub entries: usize,
    pub bytes: u64,
    pub skipped: usize,
}

/// Unpack `archive` (of kind `format`) into `dest`, which must exist and should be empty. `scratch` is a folder on the same disk
/// for intermediate files (xz). `on_progress` runs a few times a second on this thread.
pub fn extract(
    format: Format,
    archive: &Path,
    dest: &Path,
    scratch: &Path,
    limits: Limits,
    cancel: &Cancel,
    on_progress: &mut dyn FnMut(Position),
) -> Result<Stats, InstallError> {
    let total = match format {
        Format::Zip => zip_format::unpacked_size(archive),
        Format::SevenZip => sevenz_format::unpacked_size(archive),
        _ => None,
    };
    let mut sink = sink::Sink::new(dest, limits, total, cancel, on_progress);
    let result = match format {
        Format::Zip => zip_format::extract(archive, &mut sink),
        Format::TarGz => tar_format::extract_gz(archive, &mut sink),
        Format::TarXz => tar_format::extract_xz(archive, scratch, &mut sink),
        Format::SevenZip => sevenz_format::extract(archive, &mut sink),
        Format::Rar => return external_with_stats(format, archive, dest),
        other => return Err(InstallError::Unsupported(format!("{other:?} is not an archive"))),
    };
    match result {
        Ok(()) => Ok(Stats { entries: sink.entries(), bytes: sink.bytes(), skipped: sink.skipped }),
        // A zip using a compression method this reader lacks: another tool may have it. Anything else is the archive's fault.
        Err(InstallError::Archive(message)) if format == Format::Zip && message.contains("nsupported") => {
            tracing::info!(%message, "the zip reader cannot handle this archive; trying a tool from the system");
            external_with_stats(format, archive, dest)
        }
        Err(error) => Err(error),
    }
}

fn external_with_stats(format: Format, archive: &Path, dest: &Path) -> Result<Stats, InstallError> {
    external::extract(format, archive, dest)?;
    Ok(Stats::default())
}

#[cfg(test)]
pub(crate) mod tests;
