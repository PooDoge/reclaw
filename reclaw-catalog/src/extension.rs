//! Reclaw's own optional block on an app: what a storefront needs and Quiver's format has no place for. It lives under
//! one key, `reclaw`, so it can never collide with a field Quiver adds later, and Quiver (which ignores keys it does
//! not know) can read a catalog that has it. Every field is optional and a damaged block is dropped, not fatal: this
//! is decoration on an entry that is already complete without it.
use std::collections::BTreeMap;

use reclaw_games::{
    project::{Media, Requirements},
    settings::Capabilities,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Screenshot {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Video {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Extension {
    /// One line under the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// A paragraph for the game page, plain text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Wide banner art, https.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_url: Option<String>,
    /// Portrait cover art, https.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capsule_url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<Screenshot>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub videos: Vec<Video>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Requirements>,
    /// The system the game came from, when the tags do not say it clearly (a tag word such as `x360`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// The launch settings the game honors (resolution, window mode ...) and how each is applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    /// Named places on the web: `website`, `discord`, `wiki` ...
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, String>,
}

impl Extension {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Read a `reclaw` value. A block that cannot be read is left out, and the second part says why.
    pub fn from_value(value: &Value) -> (Option<Self>, Option<String>) {
        match serde_json::from_value::<Self>(value.clone()) {
            Ok(ext) if ext.is_empty() => (None, None),
            Ok(ext) => (Some(ext), None),
            Err(e) => (None, Some(format!("the reclaw block was ignored: {e}"))),
        }
    }

    /// Screenshots then videos, as the game page lists them.
    pub fn media(&self) -> Vec<Media> {
        let shots = self.screenshots.iter().map(|s| Media::Screenshot { url: s.url.clone(), caption: s.caption.clone() });
        let videos =
            self.videos.iter().map(|v| Media::Video { url: v.url.clone(), thumbnail: v.thumbnail.clone(), title: v.title.clone() });
        shots.chain(videos).collect()
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_block_round_trips_and_writes_only_what_is_set() {
        let value = json!({
            "summary": "A 3D platformer",
            "heroUrl": "https://example.com/hero.jpg",
            "screenshots": [{"url": "https://example.com/1.png", "caption": "Hub"}],
            "videos": [{"url": "https://example.com/trailer"}],
            "links": {"discord": "https://discord.gg/x"}
        });
        let (ext, problem) = Extension::from_value(&value);
        let ext = ext.expect("a valid block");
        assert_eq!(problem, None);
        assert_eq!(ext.to_value(), value, "nothing is added when it is written back");
        assert_eq!(ext.media().len(), 2);
    }

    #[test]
    fn a_damaged_or_empty_block_is_dropped_without_failing_the_entry() {
        let (ext, problem) = Extension::from_value(&json!({"screenshots": "not a list"}));
        assert!(ext.is_none());
        assert!(problem.is_some_and(|p| p.contains("ignored")));
        assert_eq!(Extension::from_value(&json!({})), (None, None), "an empty block is no block");
        assert!(Extension::from_value(&json!(42)).0.is_none());
    }

    #[test]
    fn unknown_keys_are_ignored_so_a_newer_catalog_still_reads() {
        let (ext, problem) = Extension::from_value(&json!({"summary": "s", "somethingNew": [1, 2]}));
        assert_eq!(problem, None);
        assert_eq!(ext.and_then(|e| e.summary).as_deref(), Some("s"));
    }
}
