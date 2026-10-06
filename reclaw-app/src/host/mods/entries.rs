//! What the Mods screens show for one game: the site's listing joined with the game's record of installed mods, and which mods
//! have a job running. Pure.
use reclaw_catalog::mods::{ModsConfig, normalize_path};
use reclaw_mods::{Package, Provider, Record, Source, is_update, thunderstore};
use reclaw_ui::model::{ModEntry, ModProvider, ModStatus};

pub fn ui_provider(provider: Provider) -> ModProvider {
    match provider {
        Provider::Thunderstore => ModProvider::Thunderstore,
        Provider::GameBanana => ModProvider::GameBanana,
    }
}

pub fn provider_of(provider: ModProvider) -> Provider {
    match provider {
        ModProvider::Thunderstore => Provider::Thunderstore,
        ModProvider::GameBanana => Provider::GameBanana,
    }
}

/// Whether a mod configuration says where mods go and lists a site Reclaw can use.
pub fn is_usable(config: &ModsConfig) -> bool {
    !normalize_path(&config.path).is_empty() && !Source::all_of(config).is_empty()
}

/// The library's configuration when it is usable, otherwise the catalog's (a library saved by an older program may lack it).
pub fn config_for<'a>(library: &'a ModsConfig, catalog: Option<&'a ModsConfig>) -> Option<&'a ModsConfig> {
    if is_usable(library) { Some(library) } else { catalog.filter(|c| is_usable(c)) }
}

/// Thunderstore names use `_` for spaces.
fn title_of(name: &str) -> String {
    name.replace('_', " ").trim().to_string()
}

fn matches(record: &Record, package: &Package) -> bool {
    record.is(package.provider, &package.id) || (!package.full_name.is_empty() && record.is(package.provider, &package.full_name))
}

/// The page of a mod known only from the record.
fn page_of(record: &Record, provider: Provider) -> Option<String> {
    match provider {
        Provider::Thunderstore if !record.owner.is_empty() && !record.name.is_empty() && !record.source_key.is_empty() => {
            Some(thunderstore::package_page(&record.source_key, &record.owner, &record.name))
        }
        Provider::GameBanana if !record.id.is_empty() => Some(format!("https://gamebanana.com/mods/{}", record.id)),
        _ => None,
    }
}

/// A package to install for a mod known only from the record (the site's listing could not be read, or no longer lists it).
pub fn package_from_record(record: &Record, provider: Provider) -> Package {
    let mut package = Package::named(provider, &record.source_key, &record.id, &record.owner, &record.name);
    package.full_name = record.full_name.clone();
    package.version = record.version.clone();
    package
}

fn status(installed: Option<&str>, latest: &str, busy: bool) -> ModStatus {
    match (busy, installed) {
        (true, _) => ModStatus::Installing,
        (false, Some(version)) if is_update(version, latest) => ModStatus::UpdateReady,
        (false, Some(_)) => ModStatus::Installed,
        (false, None) => ModStatus::Available,
    }
}

/// One game's mods: the installed ones first (in the record's order), then the rest of the listing in the site's order. Mods the
/// site marks deprecated or adult are left out unless installed. `busy` says whether a job is running for a mod.
pub fn entries(game_id: u32, listing: &[Package], records: &[Record], busy: &dyn Fn(Provider, &str) -> bool) -> Vec<ModEntry> {
    let mut shown: Vec<ModEntry> = Vec::new();
    for record in records {
        let Some(provider) = record.provider() else { continue };
        let entry = match listing.iter().find(|p| matches(record, p)) {
            Some(package) => from_package(game_id, package, Some(&record.version), busy(provider, &package.id)),
            None => ModEntry {
                provider: ui_provider(provider),
                id: record.id.clone(),
                game_id,
                title: title_of(record.title()),
                author: record.owner.clone(),
                summary: String::new(),
                version: record.version.clone(),
                installed_version: Some(record.version.clone()),
                downloads: 0,
                tags: Vec::new(),
                status: status(Some(&record.version), &record.version, busy(provider, &record.id)),
                icon: None,
                page_url: page_of(record, provider),
            },
        };
        if !shown.iter().any(|e| e.provider == entry.provider && e.id.eq_ignore_ascii_case(&entry.id)) {
            shown.push(entry);
        }
    }
    for package in listing {
        let installed = records.iter().any(|r| matches(r, package));
        let seen = shown.iter().any(|e| e.provider == ui_provider(package.provider) && e.id.eq_ignore_ascii_case(&package.id));
        if installed || seen || package.deprecated || package.nsfw {
            continue;
        }
        shown.push(from_package(game_id, package, None, busy(package.provider, &package.id)));
    }
    shown
}

pub fn from_package(game_id: u32, package: &Package, installed: Option<&str>, busy: bool) -> ModEntry {
    let latest = if package.version.is_empty() { installed.unwrap_or_default().to_string() } else { package.version.clone() };
    ModEntry {
        provider: ui_provider(package.provider),
        id: package.id.clone(),
        game_id,
        title: title_of(if package.name.is_empty() { &package.id } else { &package.name }),
        author: package.owner.clone(),
        summary: package.summary.clone(),
        status: status(installed, &latest, busy),
        version: latest,
        installed_version: installed.map(str::to_string),
        downloads: package.downloads,
        tags: Vec::new(),
        icon: package.icon_url.clone(),
        page_url: package.page_url.clone(),
    }
}

#[cfg(test)]
mod tests {
    use reclaw_catalog::mods::ModSource;

    use super::*;

    fn package(id: &str, version: &str) -> Package {
        let (owner, name) = id.split_once('-').unwrap_or(("", id));
        let mut p = Package::named(Provider::Thunderstore, "zelda", id, owner, name);
        p.full_name = id.to_string();
        p.version = version.to_string();
        p
    }

    fn record(id: &str, version: &str) -> Record {
        let (owner, name) = id.split_once('-').unwrap_or(("", id));
        Record {
            provider: "thunderstore".into(),
            source_key: "zelda".into(),
            id: id.into(),
            full_name: id.into(),
            owner: owner.into(),
            name: name.into(),
            version: version.into(),
            ..Record::default()
        }
    }

    #[test]
    fn installed_mods_come_first_and_know_their_update() {
        let listing = [package("A-First", "1.0.0"), package("B-Better_Camera", "2.0.0"), package("C-Gone", "1.0.0")];
        let records = [record("B-Better_Camera", "1.0.0"), record("Old-Unlisted", "0.3")];
        let found = entries(7, &listing, &records, &|_, id| id == "A-First");
        let ids: Vec<&str> = found.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["B-Better_Camera", "Old-Unlisted", "A-First", "C-Gone"]);
        assert_eq!(found[0].title, "Better Camera");
        assert_eq!(
            (found[0].status, found[0].installed_version.as_deref(), found[0].version.as_str()),
            (ModStatus::UpdateReady, Some("1.0.0"), "2.0.0")
        );
        assert_eq!(found[1].status, ModStatus::Installed, "a mod the site no longer lists still shows, to be removed");
        assert_eq!(found[1].page_url.as_deref(), Some(thunderstore::package_page("zelda", "Old", "Unlisted").as_str()));
        assert_eq!(found[2].status, ModStatus::Installing);
        assert_eq!(found[3].status, ModStatus::Available);
        assert!(found.iter().all(|e| e.game_id == 7));
    }

    #[test]
    fn deprecated_and_adult_mods_show_only_when_installed() {
        let mut old = package("A-Old", "1.0");
        old.deprecated = true;
        let mut adult = package("B-Adult", "1.0");
        adult.nsfw = true;
        let listing = [old, adult];
        assert!(entries(1, &listing, &[], &|_, _| false).is_empty());
        let shown = entries(1, &listing, &[record("A-Old", "1.0")], &|_, _| false);
        assert_eq!((shown.len(), shown[0].status), (1, ModStatus::Installed));
    }

    #[test]
    fn the_library_configuration_wins_when_it_is_usable() {
        let usable = ModsConfig {
            path: "mods".into(),
            sources: vec![ModSource { provider: "thunderstore".into(), source_url: "zelda-64-recompiled".into() }],
            ..ModsConfig::default()
        };
        let empty = ModsConfig::default();
        assert_eq!(config_for(&usable, Some(&empty)), Some(&usable));
        assert_eq!(config_for(&empty, Some(&usable)), Some(&usable));
        assert_eq!(config_for(&empty, None), None);
        let no_path = ModsConfig { path: String::new(), ..usable.clone() };
        assert!(!is_usable(&no_path));
    }

    #[test]
    fn a_record_becomes_a_package_to_reinstall() {
        let p = package_from_record(&record("Cam-Better", "1.2"), Provider::Thunderstore);
        assert_eq!(
            (p.owner.as_str(), p.name.as_str(), p.full_name.as_str(), p.source_key.as_str()),
            ("Cam", "Better", "Cam-Better", "zelda")
        );
    }
}
