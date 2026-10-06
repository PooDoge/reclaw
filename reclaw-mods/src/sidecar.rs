//! The record of what mods are installed in an app, and which files each one put there: `.quiver-mods.json` in the app's
//! folder, in Quiver's format, so a folder Quiver modded shows its mods here and the other way round.
//!
//! Quiver reads a file it cannot parse as "no mods" and its next save writes over it, which forgets every file it recorded. Here
//! such a file is an error, and nothing is written until it is fixed. Saving writes a temporary file and renames it over the old
//! one, so a crash leaves the old record or the new one, never half of one.
use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{error::ModError, source::Provider};

pub const FILE_NAME: &str = ".quiver-mods.json";

/// One installed mod (or, for a GameBanana mod installed file by file, one installed file of it).
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub provider: String,
    #[serde(default)]
    pub source_key: String,
    pub id: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub download_file_id: Option<String>,
    #[serde(default)]
    pub download_file_name: Option<String>,
    /// Paths inside the mods folder, `/`-separated.
    #[serde(default)]
    pub files: Vec<String>,
}

impl Record {
    pub fn provider(&self) -> Option<Provider> {
        Provider::from_id(&self.provider)
    }

    /// Whether this record is the mod `id` of `provider` (by id, or by Thunderstore's full name, which Quiver also matches).
    pub fn is(&self, provider: Provider, id: &str) -> bool {
        self.provider() == Some(provider) && (self.id.eq_ignore_ascii_case(id) || self.full_name.eq_ignore_ascii_case(id))
    }

    /// A short name for messages.
    pub fn title(&self) -> &str {
        [&self.name, &self.full_name, &self.id].into_iter().find(|t| !t.trim().is_empty()).map_or("another mod", String::as_str)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Document {
    #[serde(default)]
    pub mods: Vec<Record>,
}

impl Document {
    pub fn path(install_root: &Path) -> PathBuf {
        install_root.join(FILE_NAME)
    }

    /// The record in `install_root`. No file is an empty record; a file that cannot be read is an error.
    pub fn load(install_root: &Path) -> Result<Self, ModError> {
        let path = Self::path(install_root);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(ModError::Unreadable { path: path.display().to_string(), error: error.to_string() }),
        };
        // An empty file is what a crash between create and write leaves; it held nothing.
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(Self::default());
        }
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
        serde_json::from_slice(bytes).map_err(|e| ModError::Unreadable { path: path.display().to_string(), error: e.to_string() })
    }

    pub fn save(&self, install_root: &Path) -> Result<(), ModError> {
        let path = Self::path(install_root);
        let temporary = install_root.join(format!("{FILE_NAME}.tmp"));
        let text = serde_json::to_string_pretty(self).map_err(|e| ModError::BadAnswer(e.to_string()))?;
        fs::write(&temporary, text).map_err(|e| ModError::io(format!("writing {}", temporary.display()), &e))?;
        fs::rename(&temporary, &path).map_err(|e| ModError::io(format!("replacing {}", path.display()), &e))
    }

    pub fn find(&self, provider: Provider, id: &str) -> impl Iterator<Item = &Record> {
        let id = id.to_string();
        self.mods.iter().filter(move |r| r.is(provider, &id))
    }

    /// The record of one mod of one source, by Thunderstore's full name: how a dependency is recognised as installed.
    pub fn has_full_name(&self, provider: Provider, source_key: &str, full_name: &str) -> bool {
        self.mods.iter().any(|r| {
            r.provider() == Some(provider)
                && r.source_key.eq_ignore_ascii_case(source_key)
                && (r.full_name.eq_ignore_ascii_case(full_name) || r.id.eq_ignore_ascii_case(full_name))
        })
    }

    /// Which record owns the file at `path` (compared ignoring case: the folder may be on a case-insensitive disk).
    pub fn owner_of(&self, path: &str) -> Option<&Record> {
        self.mods.iter().find(|r| r.files.iter().any(|f| f.eq_ignore_ascii_case(path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quivers_file_is_read_and_written_in_its_own_shape() {
        let dir = tempfile::tempdir().expect("dir");
        let quiver = r#"{
  "mods": [
    {
      "provider": "thunderstore",
      "sourceKey": "zelda-64-recompiled",
      "id": "Cam-Better_Camera",
      "fullName": "Cam-Better_Camera",
      "owner": "Cam",
      "name": "Better_Camera",
      "version": "1.2.0",
      "downloadFileId": null,
      "downloadFileName": null,
      "files": ["better_camera.nrm"],
      "somethingNewer": true
    }
  ]
}"#;
        fs::write(Document::path(dir.path()), format!("\u{feff}{quiver}")).expect("write");
        let doc = Document::load(dir.path()).expect("reads");
        assert_eq!(doc.mods.len(), 1);
        assert!(doc.mods[0].is(Provider::Thunderstore, "cam-better_camera"));
        assert_eq!(doc.owner_of("Better_Camera.nrm").map(Record::title), Some("Better_Camera"));
        assert!(doc.has_full_name(Provider::Thunderstore, "Zelda-64-Recompiled", "Cam-Better_Camera"));
        assert!(!doc.has_full_name(Provider::GameBanana, "zelda-64-recompiled", "Cam-Better_Camera"));

        doc.save(dir.path()).expect("saves");
        let written: serde_json::Value = serde_json::from_slice(&fs::read(Document::path(dir.path())).expect("read")).expect("json");
        assert_eq!(written["mods"][0]["sourceKey"], "zelda-64-recompiled");
        assert_eq!(written["mods"][0]["files"][0], "better_camera.nrm");
        assert!(!dir.path().join(format!("{FILE_NAME}.tmp")).exists());
    }

    #[test]
    fn no_file_or_an_empty_one_is_no_mods_but_a_broken_one_is_an_error() {
        let dir = tempfile::tempdir().expect("dir");
        assert_eq!(Document::load(dir.path()), Ok(Document::default()));
        fs::write(Document::path(dir.path()), "  \n").expect("write");
        assert_eq!(Document::load(dir.path()), Ok(Document::default()));
        fs::write(Document::path(dir.path()), "{\"mods\": [").expect("write");
        assert!(matches!(Document::load(dir.path()), Err(ModError::Unreadable { .. })));
    }
}
