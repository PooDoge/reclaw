//! TOML through `toml_edit`: comments, spacing and table order survive; only the edited values change.
use toml_edit::{DocumentMut, Item, Table, TableLike, Value};

use super::ConfigEditError;
use crate::settings::plan::{ConfigValue, KeyEdit};

pub(super) fn apply(text: &str, edits: &[KeyEdit]) -> Result<String, ConfigEditError> {
    let mut doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| ConfigEditError::parse("toml", e))?;
    for edit in edits {
        set_path(doc.as_table_mut(), &edit.path, to_toml(&edit.value))?;
    }
    let out = doc.to_string();
    // toml_edit reads CRLF but prints LF; give the file back the endings it came with.
    Ok(if text.contains("\r\n") { out.replace("\r\n", "\n").replace('\n', "\r\n") } else { out })
}

fn to_toml(value: &ConfigValue) -> Value {
    match value {
        ConfigValue::Bool(b) => Value::from(*b),
        ConfigValue::Int(n) => Value::from(*n),
        ConfigValue::Float(f) => Value::from(*f),
        ConfigValue::Text(s) => Value::from(s.as_str()),
    }
}

fn set_path(root: &mut Table, path: &str, mut new: Value) -> Result<(), ConfigEditError> {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err(ConfigEditError::bad_path(path, "empty name in the path"));
    }
    let Some((last, parents)) = segments.split_last() else { return Err(ConfigEditError::bad_path(path, "empty path")) };
    let mut table: &mut dyn TableLike = root;
    for segment in parents {
        if !table.contains_key(segment) {
            // Implicit: `[Graphics.Window]` alone is written, not an empty `[Graphics]` above it.
            let mut created = Table::new();
            created.set_implicit(true);
            table.insert(segment, Item::Table(created));
        }
        let item = table.get_mut(segment).ok_or_else(|| ConfigEditError::bad_path(path, format!("`{segment}` could not be created")))?;
        let not_a_table =
            if matches!(item, Item::ArrayOfTables(_)) { "TOML paths cannot index arrays" } else { "it is a value, not a table" };
        table = item.as_table_like_mut().ok_or_else(|| ConfigEditError::bad_path(path, format!("`{segment}`: {not_a_table}")))?;
    }
    match table.get_mut(last) {
        Some(item) if item.is_value() => {
            // Keep the comment and spacing around the old value.
            if let Some(old) = item.as_value() {
                *new.decor_mut() = old.decor().clone();
            }
            *item = Item::Value(new);
        }
        Some(Item::None) | None => {
            table.insert(last, Item::Value(new));
        }
        Some(_) => return Err(ConfigEditError::bad_path(path, format!("`{last}` is a table, not a value"))),
    }
    Ok(())
}
