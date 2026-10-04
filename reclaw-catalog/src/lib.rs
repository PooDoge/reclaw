//! The catalog and the library: what Reclaw knows about the apps it can install, in the format of the Quiver
//! launcher's community catalog so both can share one catalog. Everything here is plain data in and out (no network,
//! no disk, no window), so it is tested by itself; fetching and file access belong to the host.
//!
//! * `entry.rs`, `parse.rs`, `write.rs`: one app (`AppEntry`), read leniently or strictly, written back in the order
//!   Quiver's readers expect.
//! * `library.rs`: the user's own `apps.json`, read strictly.
//! * `list.rs`: a catalog list file (`Nintendo.json`): metadata, version, apps.
//! * `index.rs`: the community `index.json` that points at the lists.
//! * `platform_index.rs`, `timestamp.rs`: the generated `platform-index.json` and the dates in it.
//! * `extension.rs`: Reclaw's optional `reclaw` block on an entry, which Quiver ignores.
//! * `mods.rs`, `display_name.rs`, `source.rs`, `normalize.rs`, `json.rs`, `error.rs`: the pieces those share.
pub mod display_name;
pub mod entry;
pub mod error;
pub mod extension;
pub mod index;
mod json;
pub mod library;
pub mod list;
pub mod mods;
pub mod normalize;
pub mod parse;
pub mod platform_index;
pub mod source;
pub mod timestamp;
pub mod write;

pub use entry::AppEntry;
pub use error::CatalogError;
pub use extension::Extension;
pub use index::{CommunityIndex, IndexSource};
pub use library::{library_to_string, parse_library};
pub use list::{CatalogList, parse_list};
pub use platform_index::{PlatformDocument, PlatformIndex};
pub use source::RepoSource;
