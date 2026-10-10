//! One app, as the catalog describes it and as the library keeps it. A single type serves both: the catalog sets the
//! fields that say what the app is, the library adds the ones that belong to this user (where it is installed, which
//! release they pinned, whether it updates itself).
use reclaw_games::platform::Platform;

use crate::{extension::Extension, mods::ModsConfig, normalize::same_key, source::RepoSource};

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AppEntry {
    // What the catalog says.
    /// The game's title. Not trimmed: it is shown and compared as written.
    pub name: String,
    /// The team, port or product this version comes from; part of what tells two ports of one game apart.
    pub project: Option<String>,
    /// `owner/name` on the hosting service; empty for an app that is not downloaded from anywhere ("manual").
    pub repository: String,
    pub source: RepoSource,
    /// The install folder's name under the apps folder; unique across a library.
    pub folder_name: String,
    pub icon_url: Option<String>,
    pub tags: Vec<String>,
    /// Marker files created in the install folder (e.g. `portable.txt`).
    pub files_to_add: Vec<String>,
    /// When one release holds several games, the part of an asset's name that picks this one.
    pub release_asset_filter: Option<String>,
    pub mods: ModsConfig,
    /// A stable id issued by the catalog's website; kept so an entry can be recognised across renames.
    pub catalog_id: Option<String>,
    /// The id of the quiverlauncher.com entry the app was added from or last linked to (Quiver 3.5's `catalogEntryId`). It does not
    /// change when the site renames things, so it links the app to its entry before the repository is compared.
    pub catalog_entry_id: Option<String>,
    /// What the site's catalog last set on this library app (Quiver 3.5's `catalog`), kept so a later change of the site's does not
    /// overwrite one the user made. Reclaw does not sync these fields yet; it keeps them so Quiver still can.
    pub catalog_snapshot: Option<crate::snapshot::CatalogSnapshot>,
    /// Reclaw's own optional block (`reclaw`): pictures, text and requirements Quiver's format has no place for.
    pub extension: Option<Extension>,

    // What belongs to this user.
    pub install_path: Option<String>,
    pub preferred_version: Option<String>,
    pub skipped_update_version: Option<String>,
    pub custom_display_name: Option<String>,
    pub auto_update: bool,
    pub defer_update_tracking: bool,
    pub linux_runner: Option<String>,
    pub linux_prefix_path: Option<String>,
    pub linux_proton_path: Option<String>,
    pub linux_custom_launch_command: Option<String>,
}

impl AppEntry {
    /// An app with no repository: files the user puts in its folder, opened from the library.
    pub fn is_manual(&self) -> bool {
        self.repository.trim().is_empty()
    }

    /// What makes the app the same app in a catalog and a library: `manual:<folder>`, or `<source>:<repository>`.
    /// Compared ignoring case.
    pub fn identity_key(&self) -> String {
        if self.is_manual() {
            format!("manual:{}", self.folder_name.trim())
        } else {
            format!("{}:{}", self.source.as_str(), self.repository.trim())
        }
    }

    /// What makes it one tile of the library: the identity plus the folder, because one repository can hold several
    /// games (told apart by `release_asset_filter`) and each gets its own folder.
    pub fn instance_key(&self) -> String {
        if self.is_manual() { self.identity_key() } else { format!("{}:{}", self.identity_key(), self.folder_name.trim()) }
    }

    /// Whether two entries are the same library tile.
    pub fn same_instance(&self, other: &Self) -> bool {
        same_key(&self.instance_key(), &other.instance_key())
    }

    /// This catalog entry as the user's first copy of it: what the catalog says, with everything that belongs to one user (where it
    /// is installed, what they pinned or skipped, their own name, their runner) cleared. A pinned `preferredVersion` an author
    /// wrote is kept: it says which release the entry is about.
    pub fn for_library(&self) -> Self {
        Self {
            install_path: None,
            skipped_update_version: None,
            custom_display_name: None,
            auto_update: false,
            defer_update_tracking: false,
            linux_runner: None,
            linux_prefix_path: None,
            linux_proton_path: None,
            linux_custom_launch_command: None,
            ..self.clone()
        }
    }

    /// The system the game came from: the one the author named in the `reclaw` block, else what the tags say (see
    /// [`Platform::from_tags`]), else `Other`.
    pub fn system(&self) -> Platform {
        let named = self.extension.as_ref().and_then(|e| e.platform.as_deref()).and_then(Platform::from_tag);
        named.or_else(|| Platform::from_tags(&self.tags)).unwrap_or(Platform::Other)
    }

    /// The title to show, by the user's chosen style.
    pub fn display_name(&self, style: crate::display_name::NameStyle) -> String {
        crate::display_name::resolve(&self.name, self.project.as_deref(), self.custom_display_name.as_deref(), style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hosted(repo: &str, folder: &str) -> AppEntry {
        AppEntry { name: "Game".into(), repository: repo.into(), folder_name: folder.into(), ..Default::default() }
    }

    #[test]
    fn a_hosted_app_is_identified_by_its_repository_and_a_tile_by_its_folder_too() {
        let a = hosted(" Owner/Repo ", " Folder ");
        assert_eq!(a.identity_key(), "github:Owner/Repo");
        assert_eq!(a.instance_key(), "github:Owner/Repo:Folder");
        let gitlab = AppEntry { source: RepoSource::Gitlab, ..hosted("o/r", "f") };
        assert_eq!(gitlab.identity_key(), "gitlab:o/r");
    }

    #[test]
    fn a_manual_app_is_identified_by_its_folder_alone() {
        let a = AppEntry { name: "Mine".into(), folder_name: " My Game ".into(), ..Default::default() };
        assert!(a.is_manual());
        assert_eq!(a.identity_key(), "manual:My Game");
        assert_eq!(a.instance_key(), "manual:My Game");
    }

    #[test]
    fn two_games_from_one_repository_are_two_tiles_but_one_identity() {
        let (red, green) =
            (hosted("mstan/FireRedLeafGreenRecomp", "PokemonFireRed"), hosted("mstan/FireRedLeafGreenRecomp", "PokemonLeafGreen"));
        assert_eq!(red.identity_key(), green.identity_key());
        assert!(!red.same_instance(&green));
        assert!(
            red.same_instance(&hosted("MSTAN/firered leafgreenrecomp".replace(' ', "").as_str(), "pokemonfirered")),
            "keys compare ignoring case"
        );
    }

    #[test]
    fn the_system_is_the_authors_word_then_the_tags_then_other() {
        let tagged = |tags: &[&str]| AppEntry { tags: tags.iter().map(|t| (*t).to_string()).collect(), ..Default::default() };
        assert_eq!(tagged(&["playstation", "ps2"]).system(), Platform::Ps2);
        assert_eq!(tagged(&["recomp"]).system(), Platform::Other);
        let named =
            AppEntry { extension: Some(Extension { platform: Some("Dreamcast".into()), ..Default::default() }), ..tagged(&["n64"]) };
        assert_eq!(named.system(), Platform::Dreamcast, "the author's word wins over the tags");
        let nonsense =
            AppEntry { extension: Some(Extension { platform: Some("nonsense".into()), ..Default::default() }), ..tagged(&["n64"]) };
        assert_eq!(nonsense.system(), Platform::N64, "an unknown word falls back to the tags");
    }

    #[test]
    fn a_catalog_entry_becomes_a_clean_first_library_entry() {
        let catalog = AppEntry {
            install_path: Some("/somewhere".into()),
            preferred_version: Some("v1".into()),
            skipped_update_version: Some("v2".into()),
            custom_display_name: Some("Mine".into()),
            auto_update: true,
            linux_runner: Some("wine".into()),
            tags: vec!["n64".into()],
            ..hosted("o/r", "Folder")
        };
        let library = catalog.for_library();
        assert_eq!((library.name.as_str(), library.repository.as_str(), library.tags.clone()), ("Game", "o/r", vec!["n64".to_string()]));
        assert_eq!(library.preferred_version.as_deref(), Some("v1"), "what the author pinned stays");
        assert!(library.install_path.is_none() && library.skipped_update_version.is_none() && library.custom_display_name.is_none());
        assert!(!library.auto_update && library.linux_runner.is_none());
        assert!(library.same_instance(&catalog), "still the same app");
    }
}
