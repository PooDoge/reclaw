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
    /// When the mod was first published and last updated, in seconds since 1970, when the listing says.
    pub created: Option<u64>,
    pub updated: Option<u64>,
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
            created: None,
            updated: None,
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

/// A moment as seconds since 1970: a number of seconds (GameBanana), or an RFC 3339 text such as `2024-03-05T14:22:31.12Z`
/// (Thunderstore). Anything else, or a moment before 1970, is `None`.
pub(crate) fn timestamp(value: Option<&serde_json::Value>) -> Option<u64> {
    match value? {
        serde_json::Value::String(s) => rfc3339_seconds(s.trim()).or_else(|| number(value)),
        _ => number(value),
    }
    .filter(|t| *t > 0)
}

/// `YYYY-MM-DDTHH:MM:SS[.frac](Z|±HH:MM)`, to seconds since 1970. Only what a site's dates use; no leap seconds.
pub(crate) fn rfc3339_seconds(text: &str) -> Option<u64> {
    let digits = |s: &str| -> Option<i64> { (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten() };
    let (date, rest) = text.split_once(['T', 't', ' '])?;
    let mut parts = date.splitn(3, '-');
    let (year, month, day) = (digits(parts.next()?)?, digits(parts.next()?)?, digits(parts.next()?)?);
    if rest.len() < 8 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (hour, minute, second) = (digits(rest.get(0..2)?)?, digits(rest.get(3..5)?)?, digits(rest.get(6..8)?)?);
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let zone = rest[8..].trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match zone {
        "" | "Z" | "z" => 0,
        _ => {
            let sign = match zone.as_bytes().first()? {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let (h, m) = zone[1..].split_once(':').unwrap_or((zone.get(1..3)?, zone.get(3..).unwrap_or("0")));
            sign * (digits(h)? * 3600 + digits(m)? * 60)
        }
    };
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hour * 3600 + minute * 60 + second - offset).ok()
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
    fn dates_come_as_seconds_or_rfc3339() {
        assert_eq!(rfc3339_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(rfc3339_seconds("2024-03-05T14:22:31.123456Z"), Some(1_709_648_551));
        assert_eq!(rfc3339_seconds("2024-03-05T16:22:31+02:00"), Some(1_709_648_551));
        assert_eq!(rfc3339_seconds("2000-02-29T00:00:00Z"), Some(951_782_400));
        assert_eq!(rfc3339_seconds("2024-13-05T00:00:00Z"), None);
        assert_eq!(rfc3339_seconds("yesterday"), None);
        assert_eq!(timestamp(Some(&json!(1_709_648_551))), Some(1_709_648_551));
        assert_eq!(timestamp(Some(&json!("2024-03-05T14:22:31Z"))), Some(1_709_648_551));
        assert_eq!(timestamp(Some(&json!(0))), None);
        assert_eq!(timestamp(None), None);
    }

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
