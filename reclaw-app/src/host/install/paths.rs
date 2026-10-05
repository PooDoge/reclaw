//! Where an app's folder is. Pure: the text of a setting or a library entry in, a path or a reason out.
//!
//! The rule is Quiver's: the folder the library entry names (`installPath`) if it names one, else the app's `folderName` inside
//! the install location. The location is the text the person typed or the Settings default, with `~` meaning their home.
use std::path::{Component, Path, PathBuf};

use reclaw_catalog::AppEntry;

/// Why a path could not be made, in a sentence for the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathProblem {
    NotAbsolute(String),
    NoHome,
    BadFolderName(String),
}

impl PathProblem {
    pub fn message(&self) -> String {
        match self {
            Self::NotAbsolute(text) => format!("\"{text}\" is not a full path"),
            Self::NoHome => "the install location starts with ~ but Reclaw does not know where your home folder is".to_string(),
            Self::BadFolderName(name) => format!("the folder name \"{name}\" cannot be used"),
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::NotAbsolute(_) | Self::NoHome => {
                "Use a full path such as /home/you/Games, or one starting with ~/. Settings > Library has the default."
            }
            Self::BadFolderName(_) => "The catalog entry's folderName must be a single plain name, not a path.",
        }
    }
}

/// `~` and `~/x` mean the home folder; anything else must already be a full path.
pub fn expand(text: &str, home: Option<&Path>) -> Result<PathBuf, PathProblem> {
    let text = text.trim();
    let expanded = if text == "~" {
        home.ok_or(PathProblem::NoHome)?.to_path_buf()
    } else if let Some(rest) = text.strip_prefix("~/") {
        home.ok_or(PathProblem::NoHome)?.join(rest)
    } else {
        PathBuf::from(text)
    };
    if text.is_empty() || !expanded.is_absolute() {
        return Err(PathProblem::NotAbsolute(text.to_string()));
    }
    Ok(clean(&expanded))
}

/// Without `.` pieces, empty pieces, trailing separators and `..` that cancel something.
fn clean(path: &Path) -> PathBuf {
    reclaw_install::layout::normalize(path)
}

/// An app's own folder name, as one plain component.
pub fn folder_name(entry: &AppEntry) -> Result<&str, PathProblem> {
    let name = entry.folder_name.trim();
    let plain = !name.is_empty()
        && name != "."
        && name != ".."
        && Path::new(name).components().count() == 1
        && matches!(Path::new(name).components().next(), Some(Component::Normal(_)));
    if plain { Ok(name) } else { Err(PathProblem::BadFolderName(entry.folder_name.clone())) }
}

/// The folder for an app inside `location`.
pub fn in_location(entry: &AppEntry, location: &Path) -> Result<PathBuf, PathProblem> {
    Ok(location.join(folder_name(entry)?))
}

/// Where the app is (or would be): its recorded folder, or its name inside the default location.
pub fn folder_of(entry: &AppEntry, default_location: &str, home: Option<&Path>) -> Result<PathBuf, PathProblem> {
    match entry.install_path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(recorded) => expand(recorded, home),
        None => in_location(entry, &expand(default_location, home)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(folder: &str, install_path: Option<&str>) -> AppEntry {
        AppEntry {
            name: "G".into(),
            repository: "o/g".into(),
            folder_name: folder.into(),
            install_path: install_path.map(String::from),
            ..AppEntry::default()
        }
    }

    const HOME: &str = "/home/deck";

    #[test]
    fn a_tilde_is_the_home_folder() {
        let home = Some(Path::new(HOME));
        assert_eq!(expand("~", home), Ok(PathBuf::from("/home/deck")));
        assert_eq!(expand("~/Reclaw/Apps", home), Ok(PathBuf::from("/home/deck/Reclaw/Apps")));
        assert_eq!(expand("  /mnt/sd/Games/  ", home), Ok(PathBuf::from("/mnt/sd/Games")));
        assert_eq!(expand("~/a/../b/./c", home), Ok(PathBuf::from("/home/deck/b/c")));
    }

    #[test]
    fn what_is_not_a_full_path_is_refused_with_a_reason() {
        let home = Some(Path::new(HOME));
        for text in ["", "   ", "Games", "./Games", "../Games", "~other/Games"] {
            assert!(matches!(expand(text, home), Err(PathProblem::NotAbsolute(_))), "{text:?}");
        }
        assert_eq!(expand("~/x", None), Err(PathProblem::NoHome));
        assert!(PathProblem::NotAbsolute("Games".into()).message().contains("\"Games\""));
        assert!(PathProblem::NoHome.hint().contains("full path"));
    }

    #[test]
    fn a_folder_name_is_one_plain_name() {
        assert_eq!(folder_name(&entry("Starfall64", None)), Ok("Starfall64"));
        assert_eq!(folder_name(&entry(" Star fall ", None)), Ok("Star fall"));
        for bad in ["", " ", ".", "..", "a/b", "../x", "/abs", "a\\b".replace('\\', "/").as_str()] {
            assert!(matches!(folder_name(&entry(bad, None)), Err(PathProblem::BadFolderName(_))), "{bad:?}");
        }
    }

    #[test]
    fn an_app_goes_in_its_own_folder_inside_the_location() {
        assert_eq!(in_location(&entry("Zelda", None), Path::new("/mnt/games")), Ok(PathBuf::from("/mnt/games/Zelda")));
    }

    #[test]
    fn a_recorded_folder_wins_over_the_default() {
        let home = Some(Path::new(HOME));
        assert_eq!(folder_of(&entry("Zelda", Some("/mnt/old/Zelda")), "~/Reclaw/Apps", home), Ok(PathBuf::from("/mnt/old/Zelda")));
        assert_eq!(
            folder_of(&entry("Zelda", Some("  ")), "~/Reclaw/Apps", home),
            Ok(PathBuf::from("/home/deck/Reclaw/Apps/Zelda")),
            "blank is none"
        );
        assert_eq!(folder_of(&entry("Zelda", None), "~/Reclaw/Apps", home), Ok(PathBuf::from("/home/deck/Reclaw/Apps/Zelda")));
    }
}
