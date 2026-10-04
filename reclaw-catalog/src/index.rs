//! The community index (`index.json`): the one small file a launcher fetches first, which says where the catalog
//! lists live and where the platform metadata is. Quiver reads this one case-insensitively and nothing else, so this
//! does too.
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    error::CatalogError,
    json::{field, text},
};

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct IndexList {
    pub id: String,
    pub name: String,
    pub description: String,
    pub location: String,
    pub remote_location: Option<String>,
    /// Declared by the format and never used by it.
    pub list_version: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CommunityIndex {
    pub version: i64,
    pub lists: Vec<IndexList>,
    pub platform_metadata_url: Option<String>,
}

/// A list the index points at, with its address resolved.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IndexSource {
    pub id: String,
    pub name: String,
    pub description: String,
    pub url: String,
}

fn is_web_address(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

impl IndexList {
    fn from_value(value: &Value) -> Result<Self, CatalogError> {
        let Value::Object(object) = value else {
            return Err(CatalogError::wrong_type("a list in the index", "an object", value));
        };
        Ok(Self {
            id: text(object, "id")?.unwrap_or_default(),
            name: text(object, "name")?.unwrap_or_default(),
            description: text(object, "description")?.unwrap_or_default(),
            location: text(object, "location")?.unwrap_or_default(),
            remote_location: text(object, "remoteLocation")?,
            list_version: text(object, "listVersion")?,
        })
    }

    /// Where to fetch the list: `remoteLocation` when it is not blank, otherwise `location` if that is a web
    /// address. A bundled path is not an address; an entry with neither has none.
    pub fn address(&self) -> Option<String> {
        let remote = self.remote_location.as_deref().map(str::trim).filter(|r| !r.is_empty());
        let local = Some(self.location.trim()).filter(|l| is_web_address(l));
        remote.or(local).map(str::to_string)
    }
}

impl CommunityIndex {
    /// Read the index. The version must be a whole number (a string is an error, as in the launcher this comes
    /// from), unknown keys are ignored, and an index that lists nothing is refused: there is nothing to subscribe to.
    pub fn parse(input: &str) -> Result<Self, CatalogError> {
        let input = input.strip_prefix('\u{feff}').unwrap_or(input);
        let root: Value = serde_json::from_str(input).map_err(|e| CatalogError::Json(e.to_string()))?;
        let Value::Object(object) = &root else {
            return Err(CatalogError::wrong_type("the index", "an object", &root));
        };
        let version = match field(object, "version") {
            None | Some(Value::Null) => 0,
            Some(Value::Number(n)) => n.as_i64().ok_or_else(|| CatalogError::shape("version must be a whole number"))?,
            // A string is not a number here, whatever .NET would do elsewhere: the index is the one place it refuses.
            Some(other) => return Err(CatalogError::wrong_type("version", "a number", other)),
        };
        let lists = match field(object, "lists") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => items.iter().map(IndexList::from_value).collect::<Result<_, _>>()?,
            Some(other) => return Err(CatalogError::wrong_type("lists", "an array", other)),
        };
        let index = Self {
            version,
            lists,
            platform_metadata_url: text(object, "platformMetadataUrl")?.map(|u| u.trim().to_string()).filter(|u| !u.is_empty()),
        };
        if index.lists.is_empty() {
            return Err(CatalogError::shape("the index lists no catalogs"));
        }
        Ok(index)
    }

    /// The lists that can be fetched, ids and names trimmed, in the index's order. Entries with no address are left
    /// out, as are repeats of an id (the first wins), since an id names a cache file.
    pub fn sources(&self) -> Vec<IndexSource> {
        let mut seen = std::collections::HashSet::new();
        self.lists
            .iter()
            .filter_map(|l| {
                let url = l.address()?;
                let id = l.id.trim().to_string();
                seen.insert(id.to_lowercase()).then(|| IndexSource {
                    id,
                    name: l.name.trim().to_string(),
                    description: l.description.trim().to_string(),
                    url,
                })
            })
            .collect()
    }
}

impl IndexSource {
    /// A name for this list's cache file. The index's id is whatever its author typed and Quiver puts it into a path
    /// as it is; here anything outside letters, digits, `-`, `_` and `.` becomes `_`, and a changed id gets a short
    /// hash so two different ids never share a file. Never empty, never `.` or `..`.
    pub fn cache_stem(&self) -> String {
        let clean: String =
            self.id.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect();
        let plain = !clean.is_empty() && clean == self.id && clean.chars().any(|c| c != '.');
        if plain {
            return clean;
        }
        let digest = Sha256::digest(self.id.as_bytes());
        let short: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
        format!("{}-{short}", clean.trim_matches('.'))
    }
}

#[cfg(test)]
mod tests;
