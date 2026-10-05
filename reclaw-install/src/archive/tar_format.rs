use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use tar::EntryType;

use super::sink::Sink;
use crate::error::InstallError;

/// A reader that counts the bytes read through it, so the position in a compressed stream can be reported.
struct Counting<R> {
    inner: R,
    count: Arc<AtomicU64>,
}

impl<R: Read> Read for Counting<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

fn open(path: &Path) -> Result<File, InstallError> {
    File::open(path).map_err(|e| InstallError::io(format!("opening {}", path.display()), &e))
}

/// Unpack a tar from `reader`. `position` says how many bytes of the (compressed) archive have been consumed.
fn unpack(reader: impl Read, position: &AtomicU64, archive_len: u64, sink: &mut Sink<'_>) -> Result<(), InstallError> {
    let mut tar = tar::Archive::new(reader);
    let entries = tar.entries().map_err(|e| InstallError::Archive(format!("this is not a readable tar archive: {e}")))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| InstallError::Archive(format!("a tar entry could not be read: {e}")))?;
        let name = entry
            .path()
            .map_err(|e| InstallError::Archive(format!("a tar entry has an unreadable name: {e}")))?
            .to_string_lossy()
            .into_owned();
        let mode = entry.header().mode().ok();
        match entry.header().entry_type() {
            EntryType::Directory => sink.dir(&name, mode)?,
            EntryType::Regular | EntryType::Continuous | EntryType::GNUSparse => sink.file(&name, mode, &mut entry)?,
            EntryType::Symlink => {
                let target = entry
                    .link_name()
                    .map_err(|e| InstallError::Archive(format!("the link {name} has an unreadable target: {e}")))?
                    .map(|t| t.to_string_lossy().into_owned())
                    .unwrap_or_default();
                sink.symlink(&name, &target)?;
            }
            // Hard links, devices, FIFOs and the extension headers the reader consumes itself.
            other => sink.skip(&name, &format!("{other:?} entries are not unpacked")),
        }
        sink.report(position.load(Ordering::Relaxed), Some(archive_len));
    }
    Ok(())
}

/// `.tar.gz` / `.tgz`. Progress is the share of the compressed file consumed.
pub fn extract_gz(archive: &Path, sink: &mut Sink<'_>) -> Result<(), InstallError> {
    let len = std::fs::metadata(archive).map(|m| m.len()).unwrap_or(0);
    let count = Arc::new(AtomicU64::new(0));
    let reader = Counting { inner: BufReader::new(open(archive)?), count: count.clone() };
    unpack(flate2::read::MultiGzDecoder::new(reader), &count, len, sink)
}

/// `.tar.xz`. The xz decoder writes rather than reads, so the tar is first unpacked from xz into a file beside the archive.
pub fn extract_xz(archive: &Path, scratch: &Path, sink: &mut Sink<'_>) -> Result<(), InstallError> {
    let tar_path = scratch.join("payload.tar");
    {
        let mut input = BufReader::new(open(archive)?);
        let mut output =
            io::BufWriter::new(File::create(&tar_path).map_err(|e| InstallError::io(format!("writing {}", tar_path.display()), &e))?);
        lzma_rs::xz_decompress(&mut input, &mut output)
            .map_err(|e| InstallError::Archive(format!("this is not a readable xz file: {e}")))?;
        io::Write::flush(&mut output).map_err(|e| InstallError::io(format!("writing {}", tar_path.display()), &e))?;
    }
    let len = std::fs::metadata(&tar_path).map(|m| m.len()).unwrap_or(0);
    let count = Arc::new(AtomicU64::new(0));
    let result = unpack(Counting { inner: BufReader::new(open(&tar_path)?), count: count.clone() }, &count, len, sink);
    // The intermediate file can be as big as the game; it does not stay.
    let _ = std::fs::remove_file(&tar_path);
    result
}
