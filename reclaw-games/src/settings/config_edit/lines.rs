//! A text file as lines, for the formats without a parser (INI, key-value). Every line keeps its own
//! text and ending, so an edit changes only the lines it names.
use super::ConfigEditError;
use crate::settings::plan::ConfigValue;

pub(super) struct Line {
    pub text: String,
    /// `""` for the last line of a file with no final newline, and for lines we add.
    ending: String,
}

pub(super) struct Lines {
    pub lines: Vec<Line>,
    /// What lines we add end with: the first ending the file uses.
    eol: String,
    /// Whether the last line ends the file with a newline (an empty file counts: a new file gets one).
    final_newline: bool,
}

impl Lines {
    pub fn parse(text: &str) -> Self {
        let lines: Vec<Line> = text
            .split_inclusive('\n')
            .map(|raw| {
                let body = raw.strip_suffix('\n').unwrap_or(raw);
                let text = body.strip_suffix('\r').unwrap_or(body);
                Line { text: text.to_string(), ending: raw[text.len()..].to_string() }
            })
            .collect();
        let eol = lines.iter().map(|l| l.ending.as_str()).find(|e| !e.is_empty()).unwrap_or("\n").to_string();
        Self { lines, eol, final_newline: text.is_empty() || text.ends_with('\n') }
    }

    pub fn insert(&mut self, at: usize, text: String) {
        self.lines.insert(at, Line { text, ending: String::new() });
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            out.push_str(&line.text);
            let last = i + 1 == self.lines.len();
            match (line.ending.as_str(), last) {
                ("", true) if !self.final_newline => {}
                ("", _) => out.push_str(&self.eol),
                (ending, _) => out.push_str(ending),
            }
        }
        out
    }
}

/// How a value is written in a text format. A line break would turn one setting into two lines.
pub(super) fn text_of(value: &ConfigValue, path: &str) -> Result<String, ConfigEditError> {
    let text = match value {
        ConfigValue::Bool(b) => b.to_string(),
        ConfigValue::Int(n) => n.to_string(),
        ConfigValue::Float(f) => format!("{f:?}"),
        ConfigValue::Text(s) => s.clone(),
    };
    if text.contains(['\n', '\r']) {
        return Err(ConfigEditError::bad_path(path, "the value contains a line break"));
    }
    Ok(text)
}

pub(super) fn is_comment(trimmed: &str) -> bool {
    trimmed.starts_with([';', '#'])
}
