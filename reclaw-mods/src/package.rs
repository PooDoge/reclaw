//! What the two sites say about a mod, in one shape: a listed [`Package`], a page of them, and the [`Download`] an install takes.
use crate::source::Provider;

/// One mod as a listing shows it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Package {
    pub provider: Provider,
    /// The source it was listed in (community slug or game number).
    pub source_key: String,
    /// Thunderstore: `Owner-Name`. GameBanana: the mod's number.
    pub id: String,
    pub owner: String,
    pub name: String,
    /// Thunderstore's `Owner-Name`, which is also how its dependencies name a package. GameBanana: `Submitter-Name`.
    pub full_name: String,
    pub summary: String,
    pub icon_url: Option<String>,
    pub page_url: Option<String>,
    pub deprecated: bool,
    pub nsfw: bool,
    pub downloads: u64,
    /// Thunderstore's ratings, GameBanana's likes.
    pub rating: u64,
    /// The newest version as far as the listing says; empty when it does not (a Thunderstore listing only hints at it).
    pub version: String,
    /// Bytes, when the listing says.
    pub size: Option<u64>,
}

impl Package {
    /// A package known only by name, with everything else blank: a dependency before it is looked up.
    pub fn named(provider: Provider, source_key: &str, id: &str, owner: &str, name: &str) -> Self {
        Self {
            provider,
            source_key: source_key.to_string(),
            id: id.to_string(),
            owner: owner.to_string(),
            name: name.to_string(),
            full_name: id.to_string(),
            summary: String::new(),
            icon_url: None,
            page_url: None,
            deprecated: false,
            nsfw: false,
            downloads: 0,
            rating: 0,
            version: String::new(),
            size: None,
        }
    }
}

/// One page of a listing.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Page {
    pub packages: Vec<Package>,
    /// The next page's number, when there is one.
    pub next: Option<u32>,
    /// How many mods the source has in all, when it says.
    pub total: Option<u64>,
}

/// The file one install takes, and what it says about itself.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Download {
    pub url: String,
    pub version: String,
    pub size: Option<u64>,
    /// The file's own name when the site gives one (GameBanana): decides the archive format before the bytes do.
    pub file_name: Option<String>,
    /// GameBanana's number for the file, recorded so an update replaces the same file.
    pub file_id: Option<String>,
    /// Thunderstore's `Owner-Name-1.2.3` strings.
    pub dependencies: Vec<String>,
}

/// The order a listing is asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Sort {
    #[default]
    MostDownloaded,
    LastUpdated,
    TopRated,
    Newest,
}

impl Sort {
    pub(crate) fn thunderstore(self) -> &'static str {
        match self {
            Self::MostDownloaded => "most-downloaded",
            Self::LastUpdated => "last-updated",
            Self::TopRated => "top-rated",
            Self::Newest => "newest",
        }
    }

    pub(crate) fn gamebanana(self) -> &'static str {
        match self {
            Self::MostDownloaded => "Generic_MostDownloaded",
            Self::LastUpdated => "Generic_NewAndUpdated",
            Self::TopRated => "Generic_MostLiked",
            Self::Newest => "Generic_Newest",
        }
    }
}

/// A JSON number, or a string holding one (GameBanana writes both).
pub(crate) fn number(value: Option<&serde_json::Value>) -> Option<u64> {
    match value? {
        serde_json::Value::Number(n) => n.as_u64().or_else(|| n.as_f64().filter(|f| *f >= 0.).map(|f| f as u64)),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// A non-blank string, trimmed.
pub(crate) fn text(value: Option<&serde_json::Value>) -> Option<String> {
    value?.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

pub(crate) fn flag(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Number(n)) => n.as_u64().is_some_and(|n| n != 0),
        _ => false,
    }
}

/// Percent-encoding for one query value or path segment.
pub(crate) fn encode(text: &str) -> String {
    text.bytes().fold(String::new(), |mut out, b| {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn numbers_come_as_numbers_or_strings() {
        assert_eq!(number(Some(&json!(12))), Some(12));
        assert_eq!(number(Some(&json!("34"))), Some(34));
        assert_eq!(number(Some(&json!(1.5))), Some(1));
        assert_eq!(number(Some(&json!(-1))), None);
        assert_eq!(number(Some(&json!("x"))), None);
        assert_eq!(number(None), None);
    }

    #[test]
    fn encoding_keeps_a_value_inside_its_place_in_the_address() {
        assert_eq!(encode("a b/c?d&e"), "a%20b%2Fc%3Fd%26e");
        assert_eq!(encode("Owner-Name_1.0~"), "Owner-Name_1.0~");
    }
}
