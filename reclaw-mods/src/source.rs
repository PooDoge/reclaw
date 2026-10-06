//! Where mods for an app are listed: a provider and the app's key there (a Thunderstore community, a GameBanana game number),
//! read from the address a catalog gives. Pure.

/// The sites Reclaw can list and install mods from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Provider {
    Thunderstore,
    GameBanana,
}

impl Provider {
    /// The id a catalog and the installed-mods file use.
    pub fn id(self) -> &'static str {
        match self {
            Self::Thunderstore => "thunderstore",
            Self::GameBanana => "gamebanana",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "thunderstore" => Some(Self::Thunderstore),
            "gamebanana" => Some(Self::GameBanana),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Thunderstore => "Thunderstore",
            Self::GameBanana => "GameBanana",
        }
    }
}

/// One place an app's mods are listed.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Source {
    pub provider: Provider,
    /// Thunderstore: the community slug, lower case. GameBanana: the game's number.
    pub key: String,
}

impl Source {
    /// Read a catalog's `{provider, sourceUrl}`. `None` for a provider Reclaw does not know or an address it cannot read; the
    /// catalog keeps such sources for a newer program.
    pub fn parse(provider: &str, url: &str) -> Option<Self> {
        let provider = Provider::from_id(provider)?;
        let key = match provider {
            Provider::Thunderstore => thunderstore_community(url)?,
            Provider::GameBanana => gamebanana_game(url)?,
        };
        Some(Self { provider, key })
    }

    /// The sources of an app's mod configuration that Reclaw can use, in the catalog's order, without repeats.
    pub fn all_of(config: &reclaw_catalog::mods::ModsConfig) -> Vec<Self> {
        let mut found: Vec<Self> = Vec::new();
        for source in &config.sources {
            match Self::parse(&source.provider, &source.source_url) {
                Some(parsed) if !found.contains(&parsed) => found.push(parsed),
                Some(_) => {}
                None => tracing::debug!(provider = %source.provider, url = %source.source_url, "a mod source Reclaw cannot read"),
            }
        }
        found
    }

    /// The page a person would open for this source's mods.
    pub fn page_url(&self) -> String {
        match self.provider {
            Provider::Thunderstore => format!("https://thunderstore.io/c/{}/", self.key),
            Provider::GameBanana => format!("https://gamebanana.com/mods/games/{}", self.key),
        }
    }
}

fn is_slug(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'))
}

/// `https://thunderstore.io/c/<slug>/...` (with or without the scheme and `www.`), or a bare slug.
fn thunderstore_community(url: &str) -> Option<String> {
    let text = url.trim().trim_end_matches('/');
    let without_scheme = text.strip_prefix("https://").or_else(|| text.strip_prefix("http://")).unwrap_or(text);
    let lower = without_scheme.to_ascii_lowercase();
    let host_and_path = lower.strip_prefix("www.").unwrap_or(&lower);
    if let Some(rest) = host_and_path.strip_prefix("thunderstore.io/c/") {
        let slug = rest.split('/').next().unwrap_or_default();
        return is_slug(slug).then(|| slug.to_string());
    }
    (is_slug(text) && !text.contains(':')).then(|| text.to_ascii_lowercase())
}

/// `https://gamebanana.com/games/<n>`, `https://gamebanana.com/mods/games/<n>`, or a bare number.
fn gamebanana_game(url: &str) -> Option<String> {
    let text = url.trim();
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if digits(text) {
        return Some(text.to_string());
    }
    let rest = text.strip_prefix("https://").or_else(|| text.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    if !matches!(host.to_ascii_lowercase().as_str(), "gamebanana.com" | "www.gamebanana.com") {
        return None;
    }
    let segments: Vec<&str> = path.split(['?', '#']).next().unwrap_or_default().split('/').filter(|s| !s.is_empty()).collect();
    let id = match segments.as_slice() {
        [first, "games", id, ..] if first.eq_ignore_ascii_case("mods") => *id,
        [first, id, ..] if first.eq_ignore_ascii_case("games") => *id,
        _ => return None,
    };
    digits(id).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use reclaw_catalog::mods::{ModSource, ModsConfig};

    use super::*;

    #[test]
    fn a_thunderstore_community_is_read_from_its_address() {
        let ts = |url| Source::parse("thunderstore", url).map(|s| s.key);
        assert_eq!(ts("https://thunderstore.io/c/zelda-64-recompiled/").as_deref(), Some("zelda-64-recompiled"));
        assert_eq!(ts("thunderstore.io/c/Banjo-Recompiled").as_deref(), Some("banjo-recompiled"));
        assert_eq!(ts("https://www.thunderstore.io/c/starship/p/x/y/").as_deref(), Some("starship"));
        assert_eq!(ts("starfox-64-recompiled").as_deref(), Some("starfox-64-recompiled"));
        for bad in ["", "https://thunderstore.io/", "https://example.com/c/x", "https://thunderstore.io/c/", "a b"] {
            assert_eq!(ts(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_gamebanana_game_is_read_from_its_address() {
        let gb = |url| Source::parse("GameBanana", url).map(|s| s.key);
        assert_eq!(gb("https://gamebanana.com/games/24290").as_deref(), Some("24290"));
        assert_eq!(gb("https://gamebanana.com/mods/games/16121?x=1").as_deref(), Some("16121"));
        assert_eq!(gb("25428").as_deref(), Some("25428"));
        for bad in ["https://gamebanana.com/mods/123", "https://evil.com/games/1", "https://gamebanana.com/games/abc", "games/1"] {
            assert_eq!(gb(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn unknown_providers_and_repeats_are_left_out() {
        let config = ModsConfig {
            path: "mods".into(),
            sources: vec![
                ModSource { provider: "thunderstore".into(), source_url: "https://thunderstore.io/c/ship-of-harkinian/".into() },
                ModSource { provider: "nexus".into(), source_url: "https://nexusmods.com/x".into() },
                ModSource { provider: "thunderstore".into(), source_url: "ship-of-harkinian".into() },
                ModSource { provider: "gamebanana".into(), source_url: "https://gamebanana.com/games/16121".into() },
            ],
            ..ModsConfig::default()
        };
        let all = Source::all_of(&config);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].page_url(), "https://thunderstore.io/c/ship-of-harkinian/");
        assert_eq!(all[1].page_url(), "https://gamebanana.com/mods/games/16121");
    }
}
