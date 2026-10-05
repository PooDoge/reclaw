use reclaw_net::AddressPolicy;
use url::Url;

pub use reclaw_net::UrlError;

/// An address Reclaw is willing to fetch. Having one is the proof that the checks passed, so the
/// rest of the crate takes this and not a string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaUrl(Url);

impl MediaUrl {
    pub fn parse(text: &str) -> Result<Self, UrlError> {
        AddressPolicy::Public.parse(text).map(Self)
    }

    /// The rules, for an address that is already parsed (the same ones the network layer applies on every redirect).
    pub fn check(url: &Url) -> Result<(), UrlError> {
        AddressPolicy::Public.check(url)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn url(&self) -> &Url {
        &self.0
    }

    pub fn host(&self) -> &str {
        self.0.host_str().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
