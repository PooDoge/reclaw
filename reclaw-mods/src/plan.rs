//! Where each file of an unpacked mod goes inside the mods folder, and whether that would replace another mod's file. Pure.
//!
//! The rules, in order:
//! 1. Files a site puts beside every package to describe it (Thunderstore's `manifest.json`, `icon.png`, `README.md`,
//!    `CHANGELOG.md`) are left out when they are at the top of the archive; deeper down they belong to the mod.
//! 2. An archive whose every file is inside one folder named like the mods folder itself (`mods/x.nrm` for a game whose mods
//!    go in `mods`) was packed relative to the game's folder: that folder is dropped, so the mod does not land in `mods/mods`.
//!    Quiver does not do this. It matters for the recomps, whose runtime opens every folder in `mods` as a mod of its own.
//! 3. With the `folderPerMod` layout, an archive with any file at its top goes into a folder named for the mod (Quiver's rule).
use reclaw_catalog::mods::ModLayout;

use crate::sidecar::Document;

/// One file to place: where it is in the unpacked tree and where it goes in the mods folder, both `/`-separated.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Placement {
    pub from: String,
    pub to: String,
}

/// What decides where files go.
#[derive(Clone, Copy, Debug)]
pub struct Rules<'a> {
    /// Names left out at the top of the archive.
    pub metadata: &'a [&'a str],
    pub layout: ModLayout,
    /// The last part of the mods folder's path (`mods` for `mods`, `Mods` for `BepInEx/Mods`).
    pub mods_folder_name: &'a str,
    /// The folder a `folderPerMod` mod goes in when it needs one.
    pub folder_name: &'a str,
}

pub fn placements(files: &[String], rules: Rules) -> Vec<Placement> {
    let payload: Vec<&String> =
        files.iter().filter(|f| f.contains('/') || !rules.metadata.iter().any(|m| m.eq_ignore_ascii_case(f))).collect();
    let wrapper = payload
        .first()
        .and_then(|f| f.split_once('/'))
        .map(|(top, _)| top)
        .filter(|top| !rules.mods_folder_name.is_empty() && top.eq_ignore_ascii_case(rules.mods_folder_name))
        .filter(|top| payload.iter().all(|f| f.split_once('/').is_some_and(|(t, _)| t == *top)));
    let stripped: Vec<(&String, String)> = payload
        .into_iter()
        .map(|f| match wrapper {
            Some(top) => (f, f[top.len() + 1..].to_string()),
            None => (f, f.clone()),
        })
        .collect();
    let wrap = rules.layout == ModLayout::FolderPerMod && stripped.iter().any(|(_, to)| !to.contains('/'));
    let prefix = if wrap { folder_name(rules.folder_name) } else { String::new() };
    stripped
        .into_iter()
        .map(|(from, to)| Placement { from: from.clone(), to: if prefix.is_empty() { to } else { format!("{prefix}/{to}") } })
        .collect()
}

/// A mod's name as one folder name: characters a file name cannot have become `_`, and dots and spaces at either end go.
pub fn folder_name(name: &str) -> String {
    let cleaned: String = name.trim().chars().map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect();
    let cleaned = cleaned.trim_matches(|c| c == '.' || c == ' ');
    if cleaned.is_empty() { "mod".to_string() } else { cleaned.to_string() }
}

/// Files of the placement that another mod's record owns, grouped by that mod: `(its title, the files)`. `replacing` is the
/// mod being installed, whose own files may be replaced.
pub fn conflicts(
    placements: &[Placement],
    document: &Document,
    replacing: &dyn Fn(&crate::sidecar::Record) -> bool,
) -> Vec<(String, Vec<String>)> {
    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    for placement in placements {
        let Some(owner) =
            document.mods.iter().filter(|r| !replacing(r)).find(|r| r.files.iter().any(|f| f.eq_ignore_ascii_case(&placement.to)))
        else {
            continue;
        };
        match found.iter_mut().find(|(title, _)| title == owner.title()) {
            Some((_, files)) => files.push(placement.to.clone()),
            None => found.push((owner.title().to_string(), vec![placement.to.clone()])),
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sidecar::Record, thunderstore::METADATA_FILES};

    fn rules(layout: ModLayout) -> Rules<'static> {
        Rules { metadata: &METADATA_FILES, layout, mods_folder_name: "mods", folder_name: "Better Camera" }
    }

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn targets(found: &[Placement]) -> Vec<&str> {
        found.iter().map(|p| p.to.as_str()).collect()
    }

    #[test]
    fn a_thunderstore_package_leaves_its_description_behind() {
        let found =
            placements(&files(&["manifest.json", "icon.png", "README.md", "better_camera.nrm", "docs/README.md"]), rules(ModLayout::Flat));
        assert_eq!(targets(&found), ["better_camera.nrm", "docs/README.md"]);
    }

    #[test]
    fn a_folder_named_like_the_mods_folder_is_dropped() {
        let found = placements(&files(&["manifest.json", "mods/a.nrm", "mods/b.nrm"]), rules(ModLayout::Flat));
        assert_eq!(targets(&found), ["a.nrm", "b.nrm"]);
        assert_eq!(found[0].from, "mods/a.nrm");
        let mixed = placements(&files(&["mods/a.nrm", "other/b.txt"]), rules(ModLayout::Flat));
        assert_eq!(targets(&mixed), ["mods/a.nrm", "other/b.txt"], "only when everything is inside it");
        let other = placements(&files(&["MyMod/a.o2r"]), rules(ModLayout::Flat));
        assert_eq!(targets(&other), ["MyMod/a.o2r"], "another folder is the mod's own");
    }

    #[test]
    fn folder_per_mod_wraps_loose_files_only() {
        let loose = placements(&files(&["a.dll", "data/b.bin"]), rules(ModLayout::FolderPerMod));
        assert_eq!(targets(&loose), ["Better Camera/a.dll", "Better Camera/data/b.bin"]);
        let packed = placements(&files(&["Pack/a.dll"]), rules(ModLayout::FolderPerMod));
        assert_eq!(targets(&packed), ["Pack/a.dll"]);
    }

    #[test]
    fn a_folder_name_is_one_safe_name() {
        assert_eq!(folder_name(" a/b:c? "), "a_b_c_");
        assert_eq!(folder_name(".."), "mod");
        assert_eq!(folder_name(" . x . "), "x");
    }

    #[test]
    fn another_mods_files_are_a_conflict_but_its_own_are_not() {
        let mut document = Document::default();
        document.mods.push(Record {
            provider: "thunderstore".into(),
            id: "A-One".into(),
            name: "One".into(),
            files: files(&["shared.nrm", "one.nrm"]),
            ..Record::default()
        });
        document.mods.push(Record {
            provider: "thunderstore".into(),
            id: "B-Two".into(),
            name: "Two".into(),
            files: files(&["two.nrm"]),
            ..Record::default()
        });
        let found = placements(&files(&["Shared.nrm", "two.nrm", "new.nrm"]), rules(ModLayout::Flat));
        let clash = conflicts(&found, &document, &|r| r.id == "B-Two");
        assert_eq!(clash, vec![("One".to_string(), vec!["Shared.nrm".to_string()])]);
        assert!(conflicts(&found, &Document::default(), &|_| false).is_empty());
    }
}
