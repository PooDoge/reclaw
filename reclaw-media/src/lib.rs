//! Everything Reclaw fetches from the internet to show: artwork, screenshots, icons and project
//! READMEs. Nothing here knows about the UI toolkit, so all of it is tested with a fake network.
//!
//! * `source`: which addresses are allowed at all (https only, nothing on the local network)
//! * `dimensions`: a picture's size, from its header
//! * `sniff`: what a downloaded file really is, whatever the server said it was
//! * `store`: the on-disk cache, written atomically and trimmed to a size
//! * `fetch`: the [`Fetch`] trait, and the real HTTP client behind it
//! * `cache`: [`MediaCache`], which decides between the store and the network
//! * `hub`: [`MediaHub`], a few background workers with a request queue the UI asks
//! * `readme`: turning a README into the blocks the UI draws (pure)
//!
//! READMEs and the images in them are written by strangers. Fetching is therefore https only, capped
//! in size and time, refuses addresses on the local network (also when a redirect leads there), and
//! never runs anything it downloads.
pub mod cache;
pub mod dimensions;
pub mod fetch;
pub mod hub;
pub mod readme;
pub mod sniff;
pub mod source;
pub mod store;

pub use cache::{Cached, MediaCache, MediaError, Policy, Want};
pub use fetch::{Fetch, FetchError, Fetched, HttpFetcher};
pub use hub::MediaHub;
pub use sniff::ImageKind;
pub use source::{MediaUrl, UrlError};
pub use store::{DiskStore, Entry, Kind};
