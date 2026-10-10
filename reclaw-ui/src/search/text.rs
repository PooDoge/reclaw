//! Matching a query: its words, whether a line has all of them, and which parts of a line to highlight.

/// A query's words, lower case. Quotes are not special: people type titles, not expressions.
pub fn terms(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// Whether every word of `query` is in one of `fields` (ignoring case). An empty query matches everything.
pub fn matches_all(fields: &[&str], query: &str) -> bool {
    let fields: Vec<String> = fields.iter().map(|f| f.to_lowercase()).collect();
    terms(query).iter().all(|term| fields.iter().any(|f| f.contains(term.as_str())))
}

/// A run of a line: highlighted when it is part of a word of the query.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Piece {
    pub text: String,
    pub hit: bool,
}

/// `text` cut into runs, the parts that match a word of `query` marked. Matching ignores case; the runs keep the line's own
/// letters. Overlapping words merge into one highlight.
pub fn highlight(text: &str, query: &str) -> Vec<Piece> {
    let lower: Vec<(usize, char)> = text.char_indices().flat_map(|(i, c)| c.to_lowercase().map(move |l| (i, l))).collect();
    let folded: String = lower.iter().map(|(_, c)| *c).collect();
    // Byte ranges of `text` to mark. Lowering can change a character's length, so positions go through `lower`.
    let mut marks: Vec<(usize, usize)> = Vec::new();
    for term in terms(query) {
        let mut from = 0;
        while let Some(at) = folded[from..].find(&term) {
            let start_char = folded[..from + at].chars().count();
            let end_char = start_char + term.chars().count();
            let start = lower.get(start_char).map_or(text.len(), |(i, _)| *i);
            let end = lower.get(end_char).map_or(text.len(), |(i, _)| *i);
            if end > start {
                marks.push((start, end));
            }
            from += at + term.len().max(1);
        }
    }
    marks.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in marks {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    let mut pieces = Vec::new();
    let mut at = 0;
    for (start, end) in merged {
        if start > at {
            pieces.push(Piece { text: text[at..start].to_string(), hit: false });
        }
        pieces.push(Piece { text: text[start..end].to_string(), hit: true });
        at = end;
    }
    if at < text.len() {
        pieces.push(Piece { text: text[at..].to_string(), hit: false });
    }
    pieces
}

/// "3 results for “zelda”": the line above a search's results.
pub fn results_line(found: usize, query: &str) -> String {
    let what = if found == 1 { "1 result".to_string() } else { format!("{found} results") };
    format!("{what} for \u{201c}{}\u{201d}", query.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_results_line_counts_and_quotes() {
        assert_eq!(results_line(1, " zelda "), "1 result for \u{201c}zelda\u{201d}");
        assert_eq!(results_line(0, "x"), "0 results for \u{201c}x\u{201d}");
    }

    fn marked(pieces: &[Piece]) -> String {
        pieces.iter().map(|p| if p.hit { format!("[{}]", p.text) } else { p.text.clone() }).collect()
    }

    #[test]
    fn every_word_must_be_somewhere() {
        assert!(matches_all(&["Free Camera", "lookaround"], "camera LOOK"));
        assert!(!matches_all(&["Free Camera"], "camera zoom"));
        assert!(matches_all(&["anything"], "   "), "no words: everything matches");
    }

    #[test]
    fn the_matching_parts_are_marked_in_the_lines_own_letters() {
        assert_eq!(marked(&highlight("Free Camera", "cam")), "Free [Cam]era");
        assert_eq!(marked(&highlight("Free Camera", "camera free")), "[Free] [Camera]");
        assert_eq!(marked(&highlight("banana", "an")), "b[anan]a", "repeats and touching runs merge");
        assert_eq!(marked(&highlight("Zelda", "")), "Zelda");
        assert_eq!(marked(&highlight("Zelda", "xyz")), "Zelda");
    }

    #[test]
    fn letters_that_change_length_when_lowered_do_not_break_the_cut() {
        // 'İ' lowers to two characters; the cut must still land on the line's own boundaries.
        assert_eq!(marked(&highlight("İstanbul Mod", "mod")), "İstanbul [Mod]");
        assert_eq!(marked(&highlight("Größe", "öß")), "Gr[öß]e");
    }
}
