//! A listed app as a catalog entry, ported from Quiver 3.5's `QuiverCatalogMapping`: the site's app becomes an entry of a list
//! file and goes through the same reader as the lists (`parse::parse_app`), so every rule of the format (tags, marker files, the
//! mods block, a manual app) applies to it unchanged and the rest of Reclaw installs it like any other.
//!
//! The listing does not say where an app's releases come from; the release status feed does (its provider and repository), so an
//! app is mapped with its line of the feed.
use serde_json::{Map, Value, json};

use super::{ReleaseStatus, SiteApp};
use crate::{
    entry::AppEntry,
    normalize::is_invalid_file_name_char,
    parse::{Mode, parse_app},
};

fn text(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

/// The app's folder: the site's, or its slug when the site gives none (Quiver's `FolderFor`). A character no file name can hold
/// becomes `_` and the name is trimmed of dots and spaces, so a slug always makes a usable folder.
pub fn folder_for(app: &SiteApp) -> String {
    if let Some(folder) = text(&app.launcher.folder_name) {
        return folder.to_string();
    }
    let cleaned: String = app.slug.trim().chars().map(|c| if is_invalid_file_name_char(c) { '_' } else { c }).collect();
    cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string()
}

/// The entry Quiver would add for `app`: `None` when it has no slug or name, or when the feed has no line for it (nothing says where
/// its releases are). A `manual` project, or one with no repository, is a manual app: its files are the person's to put in place.
pub fn to_entry(app: &SiteApp, status: Option<&ReleaseStatus>) -> Option<AppEntry> {
    let status = status?;
    text(&app.slug)?;
    let name = text(&app.name).or_else(|| text(&app.project_name))?;
    let mut o = Map::new();
    o.insert("name".into(), json!(name));
    if let Some(project) = text(&app.project_name) {
        o.insert("project".into(), json!(project));
    }
    o.insert("folderName".into(), json!(folder_for(app)));
    // An icon borrowed from the game would show another app's art in the library (Quiver leaves it out too).
    if let Some(icon) = app.icon() {
        o.insert("appIconUrl".into(), json!(icon.trim()));
    }
    o.insert("tags".into(), json!(app.tags));
    o.insert("filesToAdd".into(), json!(app.launcher.files_to_add));
    if let Some(id) = text(&app.id) {
        o.insert("catalogEntryId".into(), json!(id));
    }
    if let Some(id) = app.catalog_id.as_deref().and_then(text) {
        o.insert("catalogId".into(), json!(id));
    }
    let provider = status.provider.trim().to_ascii_lowercase();
    let repository = status.repository.as_deref().and_then(text);
    if let (true, Some(repository)) = (provider == "github" || provider == "gitlab", repository) {
        o.insert("repository".into(), json!(repository));
        if provider == "gitlab" {
            o.insert("repositorySource".into(), json!("gitlab"));
        }
        if let Some(filter) = app.launcher.release_asset_filter.as_deref().and_then(text) {
            o.insert("releaseAssetFilter".into(), json!(filter));
        }
    }
    // Only a block with somewhere to put mods means anything (Quiver's rule); one that does not read leaves the app without mods.
    if let Some(mods) = app.launcher.mods.as_ref().filter(|m| m.get("path").and_then(Value::as_str).is_some_and(|p| !p.trim().is_empty())) {
        o.insert("mods".into(), mods.clone());
    }
    let value = Value::Object(o);
    parse_app(&value, Mode::Lenient).or_else(|_| parse_app(&without_mods(value), Mode::Lenient)).ok()
}

fn without_mods(mut value: Value) -> Value {
    if let Some(o) = value.as_object_mut() {
        o.remove("mods");
    }
    value
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{mods::ModLayout, source::RepoSource};

    fn status(provider: &str, repository: Option<&str>) -> ReleaseStatus {
        ReleaseStatus {
            id: "k1".into(),
            slug: "g1r".into(),
            provider: provider.into(),
            repository: repository.map(str::to_string),
            ..Default::default()
        }
    }

    /// Quiver 3.5's own sample of a live `/apps` item, trimmed.
    fn g1r() -> SiteApp {
        serde_json::from_value(json!({
            "id": "k17env62k9jdbccmgg4hvabjk58f4dhm", "slug": "g1r", "name": "Pokemon Red / Blue / Yellow / Gold / Silver / Crystal",
            "projectName": "G1R Deluxe", "artwork": "https://example.com/cover.png", "tags": ["recreation", "GB", "pokemon"],
            "launcher": {"filesToAdd": ["portable.txt"], "folderName": "PokemonRedBlueYellowGoldSilverCrystal-Gen1RecompProject",
              "mods": {"layout": "folderPerMod", "path": "mods", "sources": [{"provider": "gamebanana", "sourceUrl": "https://gamebanana.com/games/25428"}]}}
        }))
        .expect("an app")
    }

    #[test]
    fn a_github_app_maps_to_the_entry_quiver_adds() {
        let entry = to_entry(&g1r(), Some(&status("github", Some("bryanthaboi/gen1recomp")))).expect("an entry");
        assert_eq!(entry.name, "Pokemon Red / Blue / Yellow / Gold / Silver / Crystal");
        assert_eq!(entry.project.as_deref(), Some("G1R Deluxe"));
        assert_eq!((entry.repository.as_str(), entry.source), ("bryanthaboi/gen1recomp", RepoSource::Github));
        assert_eq!(entry.folder_name, "PokemonRedBlueYellowGoldSilverCrystal-Gen1RecompProject");
        assert_eq!(entry.icon_url.as_deref(), Some("https://example.com/cover.png"));
        assert_eq!(entry.tags, ["recreation", "gb", "pokemon"], "tags as the format keeps them");
        assert_eq!(entry.files_to_add, ["portable.txt"]);
        assert_eq!(entry.catalog_entry_id.as_deref(), Some("k17env62k9jdbccmgg4hvabjk58f4dhm"));
        assert_eq!((entry.mods.path.as_str(), entry.mods.layout), ("mods", ModLayout::FolderPerMod));
        assert_eq!(entry.mods.sources[0].provider, "gamebanana");
    }

    #[test]
    fn the_provider_decides_where_releases_come_from() {
        let mut app = g1r();
        app.launcher.release_asset_filter = Some("EXIT1".into());
        let gitlab = to_entry(&app, Some(&status("gitlab", Some("sonicdcer/Starfox64Recomp")))).expect("an entry");
        assert_eq!((gitlab.source, gitlab.release_asset_filter.as_deref()), (RepoSource::Gitlab, Some("EXIT1")));
        for manual in [status("manual", None), status("manual", Some("FluffyQuack/ReXGlue-EXIT")), status("github", None)] {
            let entry = to_entry(&app, Some(&manual)).expect("an entry");
            assert!(entry.is_manual() && entry.release_asset_filter.is_none(), "{manual:?}");
        }
        assert_eq!(to_entry(&app, None), None, "nothing says where its releases are");
    }

    #[test]
    fn an_empty_folder_uses_the_slug_and_borrowed_art_is_not_the_icon() {
        let mut app = g1r();
        app.launcher.folder_name = " ".into();
        app.slug = "simcity2000:opensc2k.".into();
        app.artwork_from_game = true;
        let entry = to_entry(&app, Some(&status("github", Some("o/r")))).expect("an entry");
        assert_eq!(entry.folder_name, "simcity2000_opensc2k");
        assert_eq!(entry.icon_url, None);
    }

    #[test]
    fn a_mods_block_that_does_not_read_is_dropped_not_the_app() {
        let mut app = g1r();
        app.launcher.mods = Some(json!({"path": "mods", "layout": "folderPerMod", "sources": "not a list"}));
        let entry = to_entry(&app, Some(&status("github", Some("o/r")))).expect("still an entry");
        assert_eq!(entry.repository, "o/r");
        app.launcher.mods = Some(json!({"path": "", "sources": []}));
        assert!(to_entry(&app, Some(&status("github", Some("o/r")))).expect("an entry").mods.path.is_empty());
        app.name = " ".into();
        assert_eq!(to_entry(&app, Some(&status("github", Some("o/r")))).map(|e| e.name), Some("G1R Deluxe".into()));
    }
}
