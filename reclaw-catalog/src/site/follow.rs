//! Keeping a library app looking like its catalog entry, ported from Quiver 3.5's `CatalogLibrarySync`: its name, project, icon and
//! tags. A field follows the catalog only while it still holds what the catalog last set (the `catalog` snapshot in `apps.json`), so
//! anything the person changed stays theirs. Their display name (`customDisplayName`) is a separate field and never changes here.
//! Tags the catalog adds or drops are added or dropped; tags the person added or removed stay as they are.
use super::SiteApp;
use crate::{entry::AppEntry, normalize, snapshot::CatalogSnapshot};

fn text(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

fn same(a: Option<&str>, b: Option<&str>) -> bool {
    text(a) == text(b)
}

fn has(tags: &[String], tag: &str) -> bool {
    tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
}

/// Update `entry` from its site entry; true when anything changed (and the library should be saved).
pub fn follow(entry: &mut AppEntry, app: &SiteApp) -> bool {
    let last = entry.catalog_snapshot.clone();
    let next = CatalogSnapshot {
        name: text(Some(&app.name)),
        project: text(Some(&app.project_name)),
        // An icon borrowed from the game would show another app's art, so it is not offered (as when adding an app).
        icon_url: text(app.icon()),
        tags: normalize::tags(&app.tags),
    };
    let mut changed = false;
    // The entry's id links the app to it from now on, before its repository is compared.
    if let Some(id) = text(Some(&app.id))
        && entry.catalog_entry_id.as_deref() != Some(id.as_str())
    {
        entry.catalog_entry_id = Some(id);
        changed = true;
    }

    // Never synced: take the catalog's. Since then, only while the person has not changed it.
    let follows = |current: Option<&str>, before: Option<&str>, value: Option<&str>| {
        value.is_some() && (last.is_none() || same(current, before)) && !same(current, value)
    };
    if follows(Some(&entry.name), last.as_ref().and_then(|l| l.name.as_deref()), next.name.as_deref())
        && let Some(name) = &next.name
    {
        entry.name = name.clone();
        changed = true;
    }
    if follows(entry.project.as_deref(), last.as_ref().and_then(|l| l.project.as_deref()), next.project.as_deref()) {
        entry.project = next.project.clone();
        changed = true;
    }
    if follows(entry.icon_url.as_deref(), last.as_ref().and_then(|l| l.icon_url.as_deref()), next.icon_url.as_deref()) {
        entry.icon_url = next.icon_url.clone();
        changed = true;
    }

    let before: &[String] = last.as_ref().map_or(&[], |l| &l.tags);
    let dropped: Vec<&String> = before.iter().filter(|t| !has(&next.tags, t)).collect();
    let mut tags: Vec<String> = entry.tags.iter().filter(|t| !dropped.iter().any(|d| d.eq_ignore_ascii_case(t))).cloned().collect();
    for tag in next.tags.iter().filter(|t| !has(before, t)) {
        if !has(&tags, tag) {
            tags.push(tag.clone());
        }
    }
    if tags != entry.tags {
        entry.tags = tags;
        changed = true;
    }

    let moved = last.as_ref().is_none_or(|l| {
        !same(l.name.as_deref(), next.name.as_deref())
            || !same(l.project.as_deref(), next.project.as_deref())
            || !same(l.icon_url.as_deref(), next.icon_url.as_deref())
            || l.tags.len() != next.tags.len()
            || l.tags.iter().zip(&next.tags).any(|(a, b)| !a.eq_ignore_ascii_case(b))
    });
    if moved {
        entry.catalog_snapshot = Some(next);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(name: &str, project: &str, icon: &str, tags: &[&str]) -> SiteApp {
        SiteApp {
            id: "k1".into(),
            name: name.into(),
            project_name: project.into(),
            artwork: Some(icon.into()),
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
            ..Default::default()
        }
    }

    fn library(name: &str, tags: &[&str]) -> AppEntry {
        AppEntry {
            name: name.into(),
            project: Some("Old".into()),
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
            repository: "o/r".into(),
            folder_name: "F".into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_never_synced_app_takes_the_catalogs_fields_and_remembers_them() {
        let mut entry = library("Zelda", &["n64", "mine"]);
        assert!(follow(&mut entry, &site("Zelda 64", "Ship", "https://x/i.png", &["n64", "zelda"])));
        assert_eq!(
            (entry.name.as_str(), entry.project.as_deref(), entry.icon_url.as_deref()),
            ("Zelda 64", Some("Ship"), Some("https://x/i.png"))
        );
        assert_eq!(entry.tags, ["n64", "mine", "zelda"]);
        assert_eq!(entry.catalog_entry_id.as_deref(), Some("k1"));
        assert_eq!(entry.catalog_snapshot.as_ref().map(|s| s.tags.clone()), Some(vec!["n64".to_string(), "zelda".to_string()]));
        assert!(!follow(&mut entry, &site("Zelda 64", "Ship", "https://x/i.png", &["n64", "zelda"])), "nothing new: nothing to save");
    }

    #[test]
    fn what_the_person_changed_stays_theirs() {
        let mut entry = library("Zelda", &["n64"]);
        follow(&mut entry, &site("Zelda 64", "Ship", "https://x/i.png", &["n64", "zelda"]));
        entry.name = "My Zelda".into();
        entry.tags.retain(|t| t != "zelda");
        entry.tags.push("favorite".into());
        assert!(follow(&mut entry, &site("The Legend of Zelda", "Ship", "https://x/new.png", &["n64", "zelda", "hd"])));
        assert_eq!(entry.name, "My Zelda", "renamed by the person");
        assert_eq!(entry.icon_url.as_deref(), Some("https://x/new.png"), "untouched, so it follows");
        assert_eq!(entry.tags, ["n64", "favorite", "hd"], "a removed tag stays removed; a new one arrives");
        follow(&mut entry, &site("The Legend of Zelda", "Ship", "https://x/new.png", &["zelda", "hd"]));
        assert_eq!(entry.tags, ["favorite", "hd"], "a tag the catalog dropped goes");
    }

    #[test]
    fn a_borrowed_icon_is_never_taken() {
        let mut entry = library("Zelda", &[]);
        let mut app = site("Zelda", "", "https://x/game.png", &[]);
        app.artwork_from_game = true;
        follow(&mut entry, &app);
        assert_eq!(entry.icon_url, None);
        assert_eq!(entry.project.as_deref(), Some("Old"), "a blank project is nothing to follow");
    }
}
