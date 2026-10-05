use reclaw_catalog::CatalogError;
use reclaw_net::NetError;

/// Why the catalog could not be loaded at all. A single list or the platform metadata failing is not this: the rest is still
/// usable and the failure is recorded as a [`Problem`](crate::Problem).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SyncError {
    #[error("the community catalog index could not be loaded: {0}")]
    NoIndex(NetError),
    #[error("the community catalog index is not valid: {0}")]
    BadIndex(CatalogError),
}

impl SyncError {
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::NoIndex(e) => e.hint(),
            Self::BadIndex(_) => None,
        }
    }
}
