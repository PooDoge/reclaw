//! Pictures and documents fetched from the internet, as the UI sees them.
//!
//! The fetching, the cache on disk and every safety rule live in `reclaw-media`. This module is the
//! thin Freya side: [`use_remote_file`] turns an address into a state a component can draw
//! (loading, ready, failed), and honors the "download artwork and READMEs" setting.
//!
//! Components that use it are keyed by the address (`.key(url)`), so a different address is a new
//! component that asks afresh, and the hook only has to ask once.
mod remote;

pub use remote::{RemoteFile, use_media_enabled, use_remote_file};
