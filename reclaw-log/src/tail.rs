//! The end of the log, for the diagnostics report: what happened just before the user asked for help.
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

/// How far back from the end to look. A line is a few hundred bytes, so this is hundreds of lines.
const WINDOW: u64 = 256 * 1024;

/// The last `lines` lines of `path` (fewer if the file is shorter).
pub fn tail(path: &Path, lines: usize) -> io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let start = length.saturating_sub(WINDOW);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut all: Vec<&str> = text.lines().collect();
    if start > 0 && !all.is_empty() {
        // The window began in the middle of a line.
        all.remove(0);
    }
    let from = all.len().saturating_sub(lines);
    Ok(all[from..].iter().map(|l| (*l).to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_lines_come_back_in_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("l.log");
        std::fs::write(&path, (1..=10).map(|n| format!("line {n}\n")).collect::<String>()).expect("write");
        assert_eq!(tail(&path, 3).expect("tail"), ["line 8", "line 9", "line 10"]);
        assert_eq!(tail(&path, 99).expect("tail").len(), 10);
    }

    #[test]
    fn a_big_file_gives_whole_lines_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.log");
        std::fs::write(&path, (0..20_000).map(|n| format!("entry {n:06} {}\n", "y".repeat(30))).collect::<String>()).expect("write");
        let last = tail(&path, 5).expect("tail");
        assert_eq!(last.len(), 5);
        assert!(last[4].starts_with("entry 019999"), "{last:?}");
        assert!(last.iter().all(|l| l.starts_with("entry ") && l.ends_with(&"y".repeat(30))));
    }

    #[test]
    fn a_missing_file_is_an_error_the_caller_can_report() {
        assert!(tail(Path::new("/nonexistent/reclaw.log"), 5).is_err());
    }
}
