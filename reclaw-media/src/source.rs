use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

/// The longest address accepted. Longer ones are tracking tokens or attacks, not images.
const MAX_URL_LEN: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlError {
    #[error("not a web address: {0}")]
    Invalid(String),
    #[error("only https addresses are fetched")]
    NotHttps,
    #[error("the address has no host")]
    NoHost,
    #[error("addresses with a user name or password are not fetched")]
    Credentials,
    #[error("only the standard https port is used")]
    Port,
    #[error("addresses on the local network are not fetched")]
    LocalNetwork,
    #[error("the address is too long")]
    TooLong,
}

/// An address Reclaw is willing to fetch. Having one is the proof that the checks passed, so the
/// rest of the crate takes this and not a string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaUrl(Url);

impl MediaUrl {
    pub fn parse(text: &str) -> Result<Self, UrlError> {
        if text.len() > MAX_URL_LEN {
            return Err(UrlError::TooLong);
        }
        let url = Url::parse(text.trim()).map_err(|e| UrlError::Invalid(e.to_string()))?;
        Self::check(&url)?;
        Ok(Self(url))
    }

    /// The rules, for an address that is already parsed. The HTTP client also calls this on every
    /// redirect, so a public address cannot send the request somewhere private.
    pub fn check(url: &Url) -> Result<(), UrlError> {
        if url.as_str().len() > MAX_URL_LEN {
            return Err(UrlError::TooLong);
        }
        if url.scheme() != "https" {
            return Err(UrlError::NotHttps);
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(UrlError::Credentials);
        }
        if url.port().is_some_and(|p| p != 443) {
            return Err(UrlError::Port);
        }
        match url.host() {
            None => Err(UrlError::NoHost),
            Some(Host::Domain(name)) => domain_ok(name),
            Some(Host::Ipv4(ip)) => ipv4_ok(ip),
            Some(Host::Ipv6(ip)) => ipv6_ok(ip),
        }
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

/// Names that lead to the machine or the local network. A name with no dot is an intranet name.
fn domain_ok(name: &str) -> Result<(), UrlError> {
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    let local_suffix = [".localhost", ".local", ".internal", ".lan", ".home", ".corp", ".intranet"];
    if name == "localhost" || !name.contains('.') || local_suffix.iter().any(|s| name.ends_with(s)) {
        return Err(UrlError::LocalNetwork);
    }
    Ok(())
}

fn ipv4_ok(ip: Ipv4Addr) -> Result<(), UrlError> {
    let [a, b, ..] = ip.octets();
    let shared = a == 100 && (64..=127).contains(&b);
    let benchmark = a == 198 && (18..=19).contains(&b);
    if ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
        || shared
        || benchmark
        || a == 0
        || a >= 240
    {
        return Err(UrlError::LocalNetwork);
    }
    Ok(())
}

fn ipv6_ok(ip: Ipv6Addr) -> Result<(), UrlError> {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return ipv4_ok(v4);
    }
    let first = ip.segments()[0];
    let unique_local = first & 0xfe00 == 0xfc00;
    let link_local = first & 0xffc0 == 0xfe80;
    if ip.is_loopback() || ip.is_unspecified() || unique_local || link_local || IpAddr::V6(ip).is_multicast() {
        return Err(UrlError::LocalNetwork);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
