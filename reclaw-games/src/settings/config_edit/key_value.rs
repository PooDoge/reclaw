//! Key-value files by lines: `key value` or `key=value`, no sections. The whole path is the key.
use super::{
    ConfigEditError,
    lines::{Lines, is_comment, text_of},
};
use crate::settings::plan::KeyEdit;

pub(super) fn apply(text: &str, edits: &[KeyEdit]) -> Result<String, ConfigEditError> {
    let mut doc = Lines::parse(text);
    for edit in edits {
        set(&mut doc, &edit.path, &text_of(&edit.value, &edit.path)?)?;
    }
    Ok(doc.render())
}

struct Pair<'a> {
    key: &'a str,
    /// Whitespace, an optional `=`, whitespace: kept as the file wrote it.
    separator: &'a str,
    value_at: usize,
}

/// `None` for blank lines, comments and anything that does not start with a key.
fn parse(text: &str) -> Option<Pair<'_>> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() || is_comment(trimmed) || trimmed.starts_with("//") {
        return None;
    }
    let indent = text.len() - trimmed.len();
    let key_end = trimmed.find(|c: char| c.is_whitespace() || c == '=').unwrap_or(trimmed.len());
    if key_end == 0 {
        return None;
    }
    let after_key = &trimmed[key_end..];
    let after_space = after_key.trim_start();
    let after_equals = after_space.strip_prefix('=').unwrap_or(after_space);
    let separator_end = trimmed.len() - after_equals.trim_start().len();
    Some(Pair { key: &trimmed[..key_end], separator: &trimmed[key_end..separator_end], value_at: indent + separator_end })
}

fn set(doc: &mut Lines, key: &str, value: &str) -> Result<(), ConfigEditError> {
    if key.is_empty() || key.contains(|c: char| c.is_whitespace() || c == '=') || is_comment(key) || key.starts_with("//") {
        return Err(ConfigEditError::bad_path(key, "a key has no spaces or `=` and does not start a comment"));
    }
    // A new key copies the first separator the file uses; a lone word has none to copy.
    let separator = doc.lines.iter().filter_map(|l| parse(&l.text)).map(|p| p.separator).find(|s| !s.is_empty()).unwrap_or(" ").to_string();

    let mut found = false;
    for line in &mut doc.lines {
        let Some(pair) = parse(&line.text).filter(|p| p.key == key) else { continue };
        found = true;
        let keep = if pair.separator.is_empty() { None } else { Some(pair.value_at) };
        match keep {
            Some(at) => line.text.truncate(at),
            None => line.text.push_str(&separator),
        }
        line.text.push_str(value);
    }
    if !found {
        // After the last real line, so blank lines at the end of the file stay at the end.
        let at = doc.lines.iter().rposition(|l| !l.text.trim().is_empty()).map_or(0, |i| i + 1);
        doc.insert(at, format!("{key}{separator}{value}"));
    }
    Ok(())
}
