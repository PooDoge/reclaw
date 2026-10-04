//! INI by lines: `section.key` paths, comments and the spacing of existing lines untouched.
//! A key is replaced wherever it appears in its section; a missing one is added at the end of the
//! section, and a missing section at the end of the file.
use super::{
    ConfigEditError,
    lines::{Lines, is_comment, text_of},
};
use crate::settings::plan::KeyEdit;

const DEFAULT_SEPARATOR: &str = " = ";

pub(super) fn apply(text: &str, edits: &[KeyEdit]) -> Result<String, ConfigEditError> {
    let mut doc = Lines::parse(text);
    for edit in edits {
        set(&mut doc, &edit.path, &text_of(&edit.value, &edit.path)?)?;
    }
    Ok(doc.render())
}

enum Line<'a> {
    Blank,
    Comment,
    Section(&'a str),
    /// `separator` is everything between the key and the value (` = `); the value starts at `value_at`.
    Pair {
        key: &'a str,
        separator: &'a str,
        value_at: usize,
    },
    Other,
}

fn classify(text: &str) -> Line<'_> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Line::Blank;
    }
    if is_comment(trimmed) {
        return Line::Comment;
    }
    if let Some(name) = trimmed.strip_prefix('[').and_then(|rest| rest.split_once(']')).map(|(name, _)| name.trim()) {
        return Line::Section(name);
    }
    let Some(equals) = text.find('=') else { return Line::Other };
    let key_end = text[..equals].trim_end().len();
    let after = &text[equals + 1..];
    let value_at = text.len() - after.trim_start().len();
    Line::Pair { key: text[..key_end].trim(), separator: &text[key_end..value_at], value_at }
}

fn set(doc: &mut Lines, path: &str, value: &str) -> Result<(), ConfigEditError> {
    let (section, key) = match path.rsplit_once('.') {
        Some((section, key)) => (Some(section), key),
        None => (None, path),
    };
    check_name(path, key, "key", key.contains('=') || key.starts_with([';', '#', '[']))?;
    if let Some(section) = section {
        check_name(path, section, "section", section.contains(']'))?;
    }

    let mut current: Option<&str> = None;
    let mut matches = Vec::new();
    // The last line of the (last) target section: its header or its last key; `None` while not seen.
    let mut anchor: Option<usize> = None;
    let mut seen_section = section.is_none();
    let mut first_header = None;
    let mut section_separator: Option<String> = None;
    let mut file_separator: Option<String> = None;
    for (i, line) in doc.lines.iter().enumerate() {
        match classify(&line.text) {
            Line::Section(name) => {
                current = Some(name);
                first_header.get_or_insert(i);
                if Some(name) == section {
                    seen_section = true;
                    anchor = Some(i);
                    section_separator = None;
                }
            }
            Line::Pair { key: found, separator, .. } => {
                file_separator.get_or_insert_with(|| separator.to_string());
                if current == section {
                    anchor = Some(i);
                    section_separator = Some(separator.to_string());
                    if found == key {
                        matches.push(i);
                    }
                }
            }
            Line::Blank | Line::Comment | Line::Other => {}
        }
    }

    if !matches.is_empty() {
        for i in matches {
            let line = &mut doc.lines[i];
            if let Line::Pair { value_at, .. } = classify(&line.text) {
                line.text.truncate(value_at);
                line.text.push_str(value);
            }
        }
        return Ok(());
    }

    let separator = section_separator.or(file_separator).unwrap_or_else(|| DEFAULT_SEPARATOR.to_string());
    let new_line = format!("{key}{separator}{value}");
    match (anchor, seen_section) {
        (Some(i), _) => doc.insert(i + 1, new_line),
        (None, true) => insert_before_first_header(doc, first_header, new_line),
        (None, false) => append_section(doc, section.unwrap_or_default(), new_line),
    }
    Ok(())
}

fn check_name(path: &str, name: &str, what: &str, unusable: bool) -> Result<(), ConfigEditError> {
    if unusable || name.is_empty() || name != name.trim() || name.contains(['\n', '\r']) {
        return Err(ConfigEditError::bad_path(path, format!("`{name}` is not a usable INI {what} name")));
    }
    Ok(())
}

/// A key before any section goes after the file's leading comments and above the first header.
fn insert_before_first_header(doc: &mut Lines, first_header: Option<usize>, new_line: String) {
    let Some(header) = first_header else {
        let end = doc.lines.len();
        doc.insert(end, new_line);
        return;
    };
    let mut at = header;
    while at > 0 && doc.lines[at - 1].text.trim().is_empty() {
        at -= 1;
    }
    doc.insert(at, new_line);
    if at == header {
        doc.insert(at + 1, String::new());
    }
}

fn append_section(doc: &mut Lines, section: &str, new_line: String) {
    if doc.lines.last().is_some_and(|l| !l.text.trim().is_empty()) {
        let end = doc.lines.len();
        doc.insert(end, String::new());
    }
    let end = doc.lines.len();
    doc.insert(end, format!("[{section}]"));
    doc.insert(end + 1, new_line);
}
