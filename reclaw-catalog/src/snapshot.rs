//! What the quiverlauncher.com catalog last set on a library app: its name, project, icon and tags. Quiver 3.5 writes it to
//! `apps.json` as `"catalog": {"name", "project", "appIconUrl", "tags"}` and later moves a field with the site only while the app still
//! holds what the site last set, so a change the user made stays theirs. Reclaw reads and writes the block unchanged; a library both
//! programs share keeps it either way.
use serde_json::{Map, Value};

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CatalogSnapshot {
    pub name: Option<String>,
    pub project: Option<String>,
    pub icon_url: Option<String>,
    pub tags: Vec<String>,
}

fn text(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

impl CatalogSnapshot {
    /// The block as Quiver writes it. Anything but an object is no snapshot (Quiver reads it the same way); a field of the wrong type is
    /// left out rather than failing the library, because the block is the launcher's bookkeeping, not the user's data.
    pub fn from_value(value: Option<&Value>) -> Option<Self> {
        let object = value?.as_object()?;
        let tags = object
            .get("tags")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        Some(Self { name: text(object, "name"), project: text(object, "project"), icon_url: text(object, "appIconUrl"), tags })
    }

    /// Written with every key, nulls included, as Quiver writes it.
    pub fn to_value(&self) -> Value {
        let mut o = Map::new();
        let opt = |t: &Option<String>| t.as_ref().map_or(Value::Null, |t| Value::String(t.clone()));
        o.insert("name".into(), opt(&self.name));
        o.insert("project".into(), opt(&self.project));
        o.insert("appIconUrl".into(), opt(&self.icon_url));
        o.insert("tags".into(), Value::Array(self.tags.iter().cloned().map(Value::String).collect()));
        Value::Object(o)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_block_reads_and_writes_back_as_quiver_wrote_it() {
        let written = json!({"name": "Zelda 64", "project": null, "appIconUrl": "https://x/icon.png", "tags": ["n64", "zelda"]});
        let snapshot = CatalogSnapshot::from_value(Some(&written)).expect("an object is a snapshot");
        assert_eq!(snapshot.name.as_deref(), Some("Zelda 64"));
        assert_eq!(snapshot.project, None);
        assert_eq!(snapshot.tags, ["n64", "zelda"]);
        assert_eq!(snapshot.to_value(), written);
    }

    #[test]
    fn anything_but_an_object_is_no_snapshot_and_a_wrong_field_is_left_out() {
        assert_eq!(CatalogSnapshot::from_value(Some(&json!("text"))), None);
        assert_eq!(CatalogSnapshot::from_value(None), None);
        let odd = CatalogSnapshot::from_value(Some(&json!({"name": 3, "tags": ["a", 1]}))).expect("still an object");
        assert_eq!((odd.name, odd.tags), (None, vec!["a".to_string()]));
    }
}
