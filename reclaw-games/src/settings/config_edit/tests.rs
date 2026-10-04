//! Shared by the per-format test files under `tests/`.
use super::*;
use crate::settings::plan::ConfigValue;

mod files;
mod ini;
mod json;
mod key_value;
mod toml;

fn edit(path: &str, value: ConfigValue) -> KeyEdit {
    KeyEdit { path: path.to_string(), value }
}

fn text(path: &str, value: &str) -> KeyEdit {
    edit(path, ConfigValue::Text(value.to_string()))
}

fn int(path: &str, value: i64) -> KeyEdit {
    edit(path, ConfigValue::Int(value))
}

fn flag(path: &str, value: bool) -> KeyEdit {
    edit(path, ConfigValue::Bool(value))
}

fn run(format: ConfigFormat, input: &str, edits: &[KeyEdit]) -> String {
    apply_edits(format, input, edits).unwrap_or_else(|e| panic!("edit failed: {e}"))
}

fn fails(format: ConfigFormat, input: &str, edits: &[KeyEdit]) -> ConfigEditError {
    match apply_edits(format, input, edits) {
        Ok(out) => panic!("expected an error, got {out:?}"),
        Err(e) => e,
    }
}

/// Every line of `before` that is not in `changed` must still be there, in order.
fn unrelated_lines_survive(before: &str, after: &str, changed: &[&str]) {
    let kept: Vec<&str> = before.lines().filter(|l| !changed.contains(l)).collect();
    let mut rest = after.lines();
    for line in kept {
        assert!(rest.any(|l| l == line), "line {line:?} was lost or moved:\n{after}");
    }
}
