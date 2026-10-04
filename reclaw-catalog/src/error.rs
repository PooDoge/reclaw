/// Why a catalog, a library or a piece of one could not be read. The text says what is wrong and where, in words
/// a person can act on; nothing here is meant to be matched on except the kind.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum CatalogError {
    /// The text is not JSON at all (or is empty, or cut short).
    #[error("not valid JSON: {0}")]
    Json(String),
    /// It is JSON, but not in the shape the format has: a wrong type, a missing list, a value out of range.
    #[error("{0}")]
    Shape(String),
}

impl CatalogError {
    pub(crate) fn shape(message: impl Into<String>) -> Self {
        Self::Shape(message.into())
    }

    /// A value of the wrong JSON type for `what`.
    pub(crate) fn wrong_type(what: &str, expected: &str, found: &serde_json::Value) -> Self {
        let found = match found {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "a boolean",
            serde_json::Value::Number(_) => "a number",
            serde_json::Value::String(_) => "a string",
            serde_json::Value::Array(_) => "an array",
            serde_json::Value::Object(_) => "an object",
        };
        Self::Shape(format!("{what} must be {expected}, not {found}"))
    }
}
