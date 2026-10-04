//! Small readers over `serde_json::Value` shared by the formats that are read case-insensitively.
use serde_json::{Map, Value};

use crate::error::CatalogError;

/// The value of the first key that equals `name` ignoring ASCII case, in document order.
pub(crate) fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    object.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v)
}

/// A string, or nothing for a missing key or a null.
pub(crate) fn text(object: &Map<String, Value>, name: &str) -> Result<Option<String>, CatalogError> {
    match field(object, name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(CatalogError::wrong_type(name, "a string", other)),
    }
}

/// A whole number, written as a number or, as .NET's web defaults allow, as a string of digits.
pub(crate) fn whole_number(value: &Value, what: &str) -> Result<i64, CatalogError> {
    match value {
        Value::Number(n) => n.as_i64().ok_or_else(|| CatalogError::shape(format!("{what} must be a whole number"))),
        Value::String(s) => s.trim().parse().map_err(|_| CatalogError::shape(format!("{what} must be a whole number, not \"{s}\""))),
        other => Err(CatalogError::wrong_type(what, "a number", other)),
    }
}
