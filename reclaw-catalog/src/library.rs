//! The library file, `apps.json`: the user's own list of apps. Read strictly, because what is read is written back
//! and a quietly shortened copy would destroy the original.
use crate::{
    entry::AppEntry,
    error::CatalogError,
    parse::{Mode, parse_apps},
};

/// Read a library file's text. Accepts a UTF-8 byte order mark, a document that is a bare array, and the older
/// `standard`/`experimental`/`custom` layout; an entry of the wrong shape, an empty file or invalid JSON is an error
/// that names the problem, never a library with something missing.
pub fn parse_library(text: &str) -> Result<Vec<AppEntry>, CatalogError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.trim().is_empty() {
        return Err(CatalogError::Json("the file is empty".into()));
    }
    let root: serde_json::Value = serde_json::from_str(text).map_err(|e| CatalogError::Json(e.to_string()))?;
    parse_apps(&root, Mode::Strict).map(|(apps, _)| apps)
}

pub use crate::write::library_to_string;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_round_trips() {
        let text = r#"{"apps": [
          {"name": "A", "folderName": "A", "repository": "o/a", "tags": ["Recomp"], "autoUpdate": true, "preferredVersion": "v1"},
          {"name": "Mine", "folderName": "Mine"}
        ]}"#;
        let apps = parse_library(text).expect("a valid library");
        assert_eq!(apps.len(), 2);
        assert_eq!(parse_library(&library_to_string(&apps)), Ok(apps));
    }

    #[test]
    fn a_byte_order_mark_and_an_empty_library_are_fine() {
        assert_eq!(parse_library("\u{feff}{\"apps\": []}"), Ok(Vec::new()));
    }

    #[test]
    fn damage_is_an_error_and_never_a_shorter_library() {
        for bad in ["", "   ", "{", "{\"apps\": [", "null", "{}", r#"{"apps": [{"name": 42}]}"#, r#"{"apps": [null]}"#] {
            assert!(parse_library(bad).is_err(), "{bad:?} must not read as a library");
        }
    }

    #[test]
    fn unknown_keys_are_ignored_on_read() {
        let apps = parse_library(r#"{"apps": [{"name": "A", "folderName": "A", "futureThing": {"x": 1}}], "also": 1}"#)
            .expect("tolerates new keys");
        assert_eq!(apps.len(), 1);
    }
}
