//! An app's mod configuration: where mods go inside the install folder, whether each mod gets a folder, and which
//! providers' pages list mods for it. The same object appears in catalog lists and in the library.
use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::{error::CatalogError, normalize::is_invalid_file_name_char};

/// How the files of a mod are laid out under the mods folder.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModLayout {
    /// Straight into the mods folder.
    #[default]
    Flat,
    /// Each mod in a folder of its own, when its archive has loose files at the top.
    FolderPerMod,
}

/// A place mods for this app are listed: a provider (`thunderstore`, `gamebanana`, or one a newer catalog knows) and the
/// address of the app's page there.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ModSource {
    pub provider: String,
    pub source_url: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ModsConfig {
    /// A folder below the install folder, `/`-separated, with no `.` or `..`; empty when unset.
    pub path: String,
    pub layout: ModLayout,
    pub sources: Vec<ModSource>,
}

/// The default provider for a source that names none.
pub const DEFAULT_PROVIDER: &str = "thunderstore";

/// A mods path: `\` becomes `/`, empty segments go, and a path with a `.` or `..` segment or a character a file name
/// cannot have becomes empty (unset) rather than something partly right.
pub fn normalize_path(path: &str) -> String {
    let segments: Vec<String> = path.replace('\\', "/").split('/').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
    if segments.iter().any(|s| s == "." || s == ".." || s.chars().any(|c| c != '/' && is_invalid_file_name_char(c))) {
        return String::new();
    }
    segments.join("/")
}

/// `folderPerMod` in any case, otherwise flat.
pub fn normalize_layout(text: Option<&str>) -> ModLayout {
    match text.map(|t| t.trim().to_lowercase()).as_deref() {
        Some("folderpermod") => ModLayout::FolderPerMod,
        _ => ModLayout::Flat,
    }
}

/// A provider id: lower case, and `thunderstore` when blank. Unknown ids are kept: a newer catalog may use one.
pub fn normalize_provider(text: Option<&str>) -> String {
    match text.map(|t| t.trim().to_lowercase()) {
        Some(p) if !p.is_empty() => p,
        _ => DEFAULT_PROVIDER.to_string(),
    }
}

/// Sources with blank addresses dropped, providers normalised and repeats (same provider and address, ignoring case)
/// removed, in the order given.
pub fn normalize_sources(sources: impl IntoIterator<Item = (Option<String>, String)>) -> Vec<ModSource> {
    let mut seen = HashSet::new();
    sources
        .into_iter()
        .filter_map(|(provider, url)| {
            let url = url.trim().to_string();
            (!url.is_empty()).then(|| ModSource { provider: normalize_provider(provider.as_deref()), source_url: url })
        })
        .filter(|s| seen.insert(format!("{}|{}", s.provider, s.source_url.to_uppercase())))
        .collect()
}

impl ModsConfig {
    /// Nothing configured, which is how an app without mods is written (the key is left out).
    pub fn is_empty(&self) -> bool {
        self.path.is_empty() && self.sources.is_empty() && self.layout == ModLayout::Flat
    }

    /// Enough to browse and install mods: a folder to put them in and somewhere to find them.
    pub fn has_usable_config(&self) -> bool {
        !self.path.is_empty() && !self.sources.is_empty()
    }

    /// The same configuration, ignoring the case of the path and the order of the sources.
    pub fn equivalent(&self, other: &Self) -> bool {
        let sorted = |c: &Self| {
            let mut keys: Vec<String> = c.sources.iter().map(|s| format!("{}|{}", s.provider, s.source_url).to_uppercase()).collect();
            keys.sort();
            keys
        };
        self.path.to_uppercase() == other.path.to_uppercase() && self.layout == other.layout && sorted(self) == sorted(other)
    }

    /// Read the `mods` object of an entry. A `mods` that is not an object, a source that is not an object and a source
    /// without an address are skipped when `strict` is off; with it on they are errors, because the library is the
    /// user's own data and a half-read copy of it must not be written back.
    pub fn from_value(value: &Value, strict: bool) -> Result<Self, CatalogError> {
        let object = match value {
            Value::Null => return Ok(Self::default()),
            Value::Object(o) => o,
            other if strict => return Err(CatalogError::wrong_type("mods", "an object", other)),
            _ => return Ok(Self::default()),
        };
        let text = |key: &str| -> Result<Option<String>, CatalogError> {
            match object.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(s)) => Ok(Some(s.clone())),
                Some(other) => Err(CatalogError::wrong_type(&format!("mods.{key}"), "a string", other)),
            }
        };
        let path = normalize_path(&text("path")?.unwrap_or_default());
        let layout = normalize_layout(text("layout")?.as_deref());
        let sources = match object.get("sources") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => read_sources(items, strict)?,
            Some(other) if strict => return Err(CatalogError::wrong_type("mods.sources", "an array", other)),
            Some(_) => Vec::new(),
        };
        Ok(Self { path, layout, sources })
    }

    /// The `mods` object as it is written: `path` and `layout` only when set, `sources` only when there are some.
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        if !self.path.is_empty() {
            object.insert("path".into(), Value::String(self.path.clone()));
        }
        if self.layout == ModLayout::FolderPerMod {
            object.insert("layout".into(), Value::String("folderPerMod".into()));
        }
        if !self.sources.is_empty() {
            let sources = self
                .sources
                .iter()
                .map(|s| {
                    let mut o = Map::new();
                    o.insert("provider".into(), Value::String(s.provider.clone()));
                    o.insert("sourceUrl".into(), Value::String(s.source_url.clone()));
                    Value::Object(o)
                })
                .collect();
            object.insert("sources".into(), Value::Array(sources));
        }
        Value::Object(object)
    }
}

fn read_sources(items: &[Value], strict: bool) -> Result<Vec<ModSource>, CatalogError> {
    let mut read = Vec::new();
    for item in items {
        let Value::Object(o) = item else {
            if strict {
                return Err(CatalogError::wrong_type("a mod source", "an object", item));
            }
            continue;
        };
        let provider = match o.get("provider") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(other) if strict => return Err(CatalogError::wrong_type("a mod source's provider", "a string", other)),
            Some(_) => None,
        };
        match o.get("sourceUrl") {
            Some(Value::String(url)) if !url.trim().is_empty() => read.push((provider, url.clone())),
            // No usable address: nothing to list mods from. The library refuses such a source; a catalog skips it.
            Some(Value::String(_)) | Some(Value::Null) | None => {
                if strict {
                    return Err(CatalogError::shape("a mod source needs a sourceUrl"));
                }
            }
            Some(other) => {
                if strict {
                    return Err(CatalogError::wrong_type("a mod source's sourceUrl", "a string", other));
                }
            }
        }
    }
    Ok(normalize_sources(read))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_path_is_cleaned_or_discarded_whole() {
        assert_eq!(normalize_path(" mods "), "mods");
        assert_eq!(normalize_path("a\\b//c/"), "a/b/c");
        assert_eq!(normalize_path("/BepInEx/plugins"), "BepInEx/plugins");
        assert_eq!(normalize_path("a/../b"), "", "a parent segment discards the path");
        assert_eq!(normalize_path("./mods"), "");
        assert_eq!(normalize_path("mo:ds"), "");
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn layout_and_provider_have_their_defaults() {
        assert_eq!(normalize_layout(Some("FolderPerMod")), ModLayout::FolderPerMod);
        assert_eq!(normalize_layout(Some("anything")), ModLayout::Flat);
        assert_eq!(normalize_layout(None), ModLayout::Flat);
        assert_eq!(normalize_provider(None), "thunderstore");
        assert_eq!(normalize_provider(Some("  GameBanana ")), "gamebanana");
        assert_eq!(normalize_provider(Some("somethingnew")), "somethingnew", "unknown providers are kept");
    }

    #[test]
    fn sources_lose_blanks_and_repeats_but_keep_their_order() {
        let sources = normalize_sources([
            (None, "https://thunderstore.io/c/a/".into()),
            (Some("GameBanana".into()), "https://gamebanana.com/games/1".into()),
            (Some("thunderstore".into()), "HTTPS://THUNDERSTORE.IO/C/A/".into()),
            (None, "  ".into()),
        ]);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].provider, "thunderstore");
        assert_eq!(sources[1].provider, "gamebanana");
    }

    #[test]
    fn equivalence_ignores_source_order_and_path_case_but_not_the_layout() {
        let a = ModsConfig::from_value(&json!({"path": "Mods", "sources": [{"provider": "thunderstore", "sourceUrl": "u1"}, {"provider": "gamebanana", "sourceUrl": "u2"}]}), false).unwrap_or_default();
        let b = ModsConfig::from_value(
            &json!({"path": "mods", "sources": [{"provider": "gamebanana", "sourceUrl": "u2"}, {"sourceUrl": "u1"}]}),
            false,
        )
        .unwrap_or_default();
        assert!(a.equivalent(&b));
        let c = ModsConfig { layout: ModLayout::FolderPerMod, ..b.clone() };
        assert!(!a.equivalent(&c));
        assert!(a.has_usable_config() && !ModsConfig { path: "mods".into(), ..Default::default() }.has_usable_config());
    }

    #[test]
    fn reading_is_forgiving_for_catalogs_and_strict_for_the_library() {
        let odd = json!({"path": "mods", "sources": ["nope", {"provider": 7, "sourceUrl": "u"}, {"provider": "gamebanana"}, {"sourceUrl": "ok"}]});
        let lenient = ModsConfig::from_value(&odd, false).unwrap_or_default();
        assert_eq!(lenient.sources.len(), 2, "the object with a number for a provider and the one with an address are kept");
        assert!(ModsConfig::from_value(&odd, true).is_err());
        assert_eq!(ModsConfig::from_value(&json!("text"), false), Ok(ModsConfig::default()));
        assert!(ModsConfig::from_value(&json!("text"), true).is_err());
        assert!(ModsConfig::from_value(&json!({"path": 3}), true).is_err());
    }

    #[test]
    fn it_writes_only_what_is_set_in_the_order_path_layout_sources() {
        let config = ModsConfig {
            path: "mods".into(),
            layout: ModLayout::FolderPerMod,
            sources: vec![ModSource { provider: "thunderstore".into(), source_url: "u".into() }],
        };
        let value = config.to_value();
        assert_eq!(
            value.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
            Some(vec!["path".into(), "layout".into(), "sources".into()])
        );
        assert_eq!(ModsConfig::from_value(&value, true), Ok(config));
        assert_eq!(ModsConfig::default().to_value(), json!({}));
    }
}
