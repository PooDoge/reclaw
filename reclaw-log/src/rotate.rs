//! The log file: appended to, and moved aside when it gets big, so a long session or a retry loop cannot fill the disk and the last
//! few runs are always there. `reclaw.log` is the current one; `reclaw.1.log` is the one before it, and so on up to `keep - 1`.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct RotatingFile {
    dir: PathBuf,
    stem: String,
    max_bytes: u64,
    /// Files kept, the current one included.
    keep: usize,
    file: File,
    size: u64,
    /// Moving the file aside failed once: say so once (on stderr, the only other place there is) and keep writing to the same file.
    rotation_failed: bool,
}

impl RotatingFile {
    /// Open `dir/stem.log` for appending, creating the folder. A file already past `max_bytes` is moved aside first.
    pub fn open(dir: &Path, stem: &str, max_bytes: u64, keep: usize) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let keep = keep.max(1);
        let (file, size) = Self::open_current(dir, stem)?;
        let mut this = Self { dir: dir.to_path_buf(), stem: stem.to_string(), max_bytes, keep, file, size, rotation_failed: false };
        if this.size >= this.max_bytes {
            this.rotate()?;
        }
        Ok(this)
    }

    fn open_current(dir: &Path, stem: &str) -> io::Result<(File, u64)> {
        let file = OpenOptions::new().create(true).append(true).open(dir.join(format!("{stem}.log")))?;
        let size = file.metadata()?.len();
        Ok((file, size))
    }

    pub fn current_path(&self) -> PathBuf {
        self.dir.join(format!("{}.log", self.stem))
    }

    fn numbered(&self, n: usize) -> PathBuf {
        self.dir.join(format!("{}.{n}.log", self.stem))
    }

    fn rotate(&mut self) -> io::Result<()> {
        // The oldest goes, every other one moves up a place, the current one becomes number 1.
        if self.keep > 1 {
            let oldest = self.numbered(self.keep - 1);
            if oldest.exists() {
                fs::remove_file(&oldest)?;
            }
            for n in (1..self.keep - 1).rev() {
                let from = self.numbered(n);
                if from.exists() {
                    fs::rename(&from, self.numbered(n + 1))?;
                }
            }
            fs::rename(self.current_path(), self.numbered(1))?;
        } else {
            fs::remove_file(self.current_path())?;
        }
        let (file, size) = Self::open_current(&self.dir, &self.stem)?;
        self.file = file;
        self.size = size;
        Ok(())
    }
}

impl Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Rotation is only tried when this line would pass the limit; if it fails the file simply keeps growing, said once.
        if self.size > 0
            && self.size + buf.len() as u64 > self.max_bytes
            && let Err(error) = self.rotate()
            && !self.rotation_failed
        {
            self.rotation_failed = true;
            eprintln!("reclaw: the log file could not be rotated ({error}); it keeps growing");
        }
        self.file.write_all(buf)?;
        self.size += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests;
