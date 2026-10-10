//! Keeping what the program knows in step with the world, without any UI: the community catalog (the index, the lists and the
//! platform metadata, loaded in parallel, saved to disk, usable offline) and the user's library file (`apps.json`, written
//! atomically with backups and never over a read error).
//!
//! * `catalog`: [`CatalogSync`], `refresh` (network) and `saved` (disk only)
//! * `snapshot`: [`CatalogSnapshot`], the result, and the merged list of [`CatalogApp`]s
//! * `library`: [`LibraryStore`]
//! * `site`: [`SiteClient`], asking quiverlauncher.com (the catalog Quiver moved to) for its listing, an app's page, reviews and releases
//! * `error`: what can go wrong, as values
pub mod catalog;
pub mod error;
pub mod library;
pub mod site;
pub mod snapshot;

pub use catalog::{CatalogSync, DEFAULT_INDEX_URL};
pub use error::SyncError;
pub use library::{LibraryError, LibraryStore};
pub use site::{SiteClient, SiteError};
pub use snapshot::{CatalogApp, CatalogSnapshot, ListSnapshot, Origin, PlatformSnapshot, Problem};
