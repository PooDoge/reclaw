//! The status badges at the top of a README ("build passing", "license MIT"). They are SVG images
//! served by a handful of sites, and the toolkit's SVG drawing has no fonts, so the text would
//! be missing. The label, message and color are in the address of the common ones, so the badge is
//! read from there and drawn by the UI as a chip. Any other badge is a chip with its alt text.
use url::Url;

/// An RGB color. Not the UI's color type: this crate knows nothing of the toolkit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadgeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl BadgeColor {
    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Whether dark text reads better on this color than light text.
    pub fn wants_dark_text(self) -> bool {
        // Relative luminance, with the usual weights; above the middle, black text wins.
        let lum = 0.2126 * f32::from(self.r) + 0.7152 * f32::from(self.g) + 0.0722 * f32::from(self.b);
        lum > 150.
    }

    /// A color by shields.io's name or as hex (`4c1`, `#44cc11`).
    fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches('#').to_ascii_lowercase();
        Some(match text.as_str() {
            "brightgreen" | "success" => Self::new(0x44, 0xcc, 0x11),
            "green" => Self::new(0x97, 0xca, 0x00),
            "yellowgreen" => Self::new(0xa4, 0xa6, 0x1d),
            "yellow" => Self::new(0xdf, 0xb3, 0x17),
            "orange" | "important" => Self::new(0xfe, 0x7d, 0x37),
            "red" | "critical" => Self::new(0xe0, 0x5d, 0x44),
            "blue" | "informational" => Self::new(0x00, 0x7e, 0xc6),
            "lightgrey" | "lightgray" | "inactive" => Self::new(0x9f, 0x9f, 0x9f),
            "grey" | "gray" => Self::new(0x55, 0x55, 0x55),
            hex if hex.chars().all(|c| c.is_ascii_hexdigit()) && matches!(hex.len(), 3 | 6) => {
                let digit = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok();
                let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
                if hex.len() == 3 {
                    Self::new(digit(0)? * 17, digit(1)? * 17, digit(2)? * 17)
                } else {
                    Self::new(pair(0)?, pair(2)?, pair(4)?)
                }
            }
            _ => return None,
        })
    }
}

/// What a badge says, ready to be drawn as a two-part chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badge {
    pub label: Option<String>,
    pub message: String,
    /// The message's background. The label's is always the neutral grey.
    pub color: BadgeColor,
}

/// Sites that serve badges. A path ending in `badge.svg` is one on any site.
const BADGE_HOSTS: &[&str] = &[
    "img.shields.io",
    "shields.io",
    "badgen.net",
    "flat.badgen.net",
    "badge.fury.io",
    "travis-ci.org",
    "travis-ci.com",
    "api.travis-ci.com",
    "codecov.io",
    "coveralls.io",
    "api.codacy.com",
    "app.codacy.com",
    "snyk.io",
    "badges.gitter.im",
    "sonarcloud.io",
];

const DEFAULT_COLOR: BadgeColor = BadgeColor::new(0x9f, 0x9f, 0x9f);

impl Badge {
    /// The badge `src` is, or `None` when it is an ordinary picture. `alt` is what a badge whose
    /// words are not in its address says instead.
    pub fn from_image(src: &str, alt: &str) -> Option<Self> {
        let url = Url::parse(src).ok()?;
        let host = url.host_str()?.to_ascii_lowercase();
        let path = url.path();
        let named_badge = path.ends_with("badge.svg") || path.ends_with("/badge") || path.contains("/badges/");
        if !BADGE_HOSTS.contains(&host.as_str()) && !named_badge {
            return None;
        }
        Some(static_badge(&url).unwrap_or_else(|| Self { label: None, message: alt_or_host(alt, &host), color: DEFAULT_COLOR }))
    }
}

fn alt_or_host(alt: &str, host: &str) -> String {
    let alt = alt.trim();
    if alt.is_empty() { host.to_string() } else { alt.to_string() }
}

/// `/badge/<label>-<message>-<color>` or `/badge/<message>-<color>`, with `--` for a dash, `__`
/// for an underscore, and `_` or `%20` for a space. `?label=` and `?color=` override.
fn static_badge(url: &Url) -> Option<Badge> {
    let path = url.path().trim_start_matches('/');
    let content = path.strip_prefix("badge/")?;
    let content = content.rsplit_once('.').filter(|(_, ext)| matches!(*ext, "svg" | "png")).map_or(content, |(stem, _)| stem);
    let decoded = percent_decode(content);
    let parts = split_dashes(&decoded);
    let (label, message, color) = match parts.as_slice() {
        [label, message, color] => (Some(label.clone()), message.clone(), BadgeColor::parse(color)),
        [message, color] => (None, message.clone(), BadgeColor::parse(color)),
        [message] => (None, message.clone(), None),
        _ => return None,
    };
    let query = |key: &str| url.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned());
    Some(Badge {
        label: query("label").or(label).map(|l| l.trim().to_string()).filter(|l| !l.is_empty()),
        message: message.trim().to_string(),
        color: query("color").and_then(|c| BadgeColor::parse(&c)).or(color).unwrap_or(DEFAULT_COLOR),
    })
}

/// Split on single dashes; a doubled dash is a literal one. Underscores become spaces unless doubled.
fn split_dashes(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('-', Some('-')) => {
                chars.next();
                current.push('-');
            }
            ('-', _) => parts.push(std::mem::take(&mut current)),
            ('_', Some('_')) => {
                chars.next();
                current.push('_');
            }
            ('_', _) => current.push(' '),
            (other, _) => current.push(other),
        }
    }
    parts.push(current);
    parts
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = text.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests;
