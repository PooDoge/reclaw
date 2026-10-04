//! JSON: parse, set dotted paths, print with the file's own indentation. Key order is kept by
//! serde_json's `preserve_order`; comments are not JSON and make the file an error.
use serde::Serialize;
use serde_json::{
    Map, Number, Value,
    ser::{PrettyFormatter, Serializer},
};

use super::ConfigEditError;
use crate::settings::plan::{ConfigValue, KeyEdit};

pub(super) fn apply(text: &str, edits: &[KeyEdit]) -> Result<String, ConfigEditError> {
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(body) => ("\u{feff}", body),
        None => ("", text),
    };
    let mut root = if body.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(body).map_err(|e| ConfigEditError::parse("json", e))?
    };
    for edit in edits {
        set_path(&mut root, &edit.path, to_json(&edit.value, &edit.path)?)?;
    }
    let mut out = print(&root, indent_of(body))?;
    let crlf = body.contains("\r\n");
    if crlf {
        // Safe because a string cannot hold a raw line break in JSON; they are all layout.
        out = out.replace('\n', "\r\n");
    }
    if body.ends_with('\n') {
        out.push_str(if crlf { "\r\n" } else { "\n" });
    }
    Ok(format!("{bom}{out}"))
}

/// A tab, 4 spaces or 2 spaces from the first indented line; 2 for a file with none (or any other width).
fn indent_of(text: &str) -> &'static str {
    let Some(line) = text.lines().find(|l| l.starts_with([' ', '\t']) && !l.trim().is_empty()) else { return "  " };
    if line.starts_with('\t') {
        "\t"
    } else if line.len() - line.trim_start_matches(' ').len() == 4 {
        "    "
    } else {
        "  "
    }
}

fn print(root: &Value, indent: &str) -> Result<String, ConfigEditError> {
    let mut buffer = Vec::new();
    let mut serializer = Serializer::with_formatter(&mut buffer, PrettyFormatter::with_indent(indent.as_bytes()));
    root.serialize(&mut serializer).map_err(|e| ConfigEditError::parse("json", e))?;
    String::from_utf8(buffer).map_err(|e| ConfigEditError::parse("json", e))
}

fn to_json(value: &ConfigValue, path: &str) -> Result<Value, ConfigEditError> {
    Ok(match value {
        ConfigValue::Bool(b) => Value::Bool(*b),
        ConfigValue::Int(n) => Value::Number((*n).into()),
        ConfigValue::Float(f) => {
            Value::Number(Number::from_f64(*f).ok_or_else(|| ConfigEditError::bad_path(path, "JSON has no NaN or infinity"))?)
        }
        ConfigValue::Text(s) => Value::String(s.clone()),
    })
}

fn set_path(root: &mut Value, path: &str, new: Value) -> Result<(), ConfigEditError> {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err(ConfigEditError::bad_path(path, "empty name in the path"));
    }
    let Some((last, parents)) = segments.split_last() else { return Err(ConfigEditError::bad_path(path, "empty path")) };
    let mut current = root;
    for segment in parents {
        current = child(current, segment, path)?;
    }
    match current {
        Value::Object(map) => {
            // Replacing keeps the key where it was.
            map.insert((*last).to_string(), new);
        }
        Value::Array(items) => match array_index(last, items.len(), path)? {
            i if i == items.len() => items.push(new),
            i => items[i] = new,
        },
        other => return Err(ConfigEditError::bad_path(path, format!("`{last}` would go inside {}", describe(other)))),
    }
    Ok(())
}

/// The container at `segment` inside `current`, created (as an object) when missing or null.
fn child<'a>(current: &'a mut Value, segment: &str, path: &str) -> Result<&'a mut Value, ConfigEditError> {
    let slot = match current {
        Value::Object(map) => map.entry(segment.to_string()).or_insert(Value::Null),
        Value::Array(items) => {
            let index = array_index(segment, items.len(), path)?;
            if index == items.len() {
                items.push(Value::Null);
            }
            &mut items[index]
        }
        other => return Err(ConfigEditError::bad_path(path, format!("`{segment}` would go inside {}", describe(other)))),
    };
    if slot.is_null() {
        *slot = Value::Object(Map::new());
    }
    if matches!(slot, Value::Object(_) | Value::Array(_)) {
        Ok(slot)
    } else {
        Err(ConfigEditError::bad_path(path, format!("`{segment}` is {}, not an object", describe(slot))))
    }
}

/// An existing index, or the length (which appends).
fn array_index(segment: &str, len: usize, path: &str) -> Result<usize, ConfigEditError> {
    let index: usize = segment.parse().map_err(|_| ConfigEditError::bad_path(path, format!("`{segment}` is not an array index")))?;
    if index > len {
        return Err(ConfigEditError::bad_path(path, format!("index {index} is past the end of an array of {len}")));
    }
    Ok(index)
}

fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}
