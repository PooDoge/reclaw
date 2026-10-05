//! Taking credentials out of text before it is written. Three layers, because each misses something the others catch: values that
//! were registered (any shape), the well-known token prefixes (a token nobody registered, pasted into an error message by a
//! server), and the places credentials travel (an `Authorization` header, `user:password@` in an address, `?access_token=`).
//! The cost of a false positive is a hidden word in a log; the cost of a miss is a leaked credential.
use std::borrow::Cow;

use crate::secret::{REDACTED, registered};

/// Token prefixes: GitHub (personal, OAuth, user-to-server, server-to-server, refresh, fine-grained) and GitLab (personal, trigger,
/// runner, deploy).
const PREFIXES: &[&str] = &["github_pat_", "ghp_", "gho_", "ghu_", "ghs_", "ghr_", "glpat-", "glptt-", "glrt-", "gldt-", "glcbt-"];
/// A prefix followed by fewer token characters than this is a word that happens to start the same way.
const MIN_TOKEN_TAIL: usize = 12;
/// Headers whose whole value is a credential.
const HEADERS: &[&str] = &["authorization:", "proxy-authorization:", "private-token:", "x-api-key:", "x-auth-token:"];
/// Query parameters whose value is a credential.
const PARAMS: &[&str] = &["access_token=", "private_token=", "token=", "api_key=", "apikey=", "password=", "secret=", "client_secret="];

/// The byte offset of the first `needle` (already lower case, ASCII) at or after `from`, ignoring the case of ASCII letters.
/// Byte offsets stay valid for `text` because only ASCII letters are folded.
fn find_ci(text: &str, needle: &str, from: usize) -> Option<usize> {
    let (haystack, needle) = (text.as_bytes().get(from..)?, needle.as_bytes());
    haystack.windows(needle.len()).position(|window| window.eq_ignore_ascii_case(needle)).map(|at| from + at)
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// `text` with credentials replaced by [`REDACTED`]. Borrows when there was nothing to remove, which is nearly always.
pub fn scrub(text: &str) -> Cow<'_, str> {
    let mut out: Cow<'_, str> = Cow::Borrowed(text);
    for secret in registered() {
        if out.contains(&secret) {
            out = Cow::Owned(out.replace(&secret, REDACTED));
        }
    }
    let rules: [fn(&str) -> Option<String>; 4] = [prefixed_tokens, header_values, url_userinfo, query_values];
    for rule in rules {
        if let Some(changed) = rule(&out) {
            out = Cow::Owned(changed);
        }
    }
    out
}

/// `ghp_AbC...` anywhere, when what follows the prefix is long enough to be a token.
fn prefixed_tokens(text: &str) -> Option<String> {
    if !PREFIXES.iter().any(|p| text.contains(p)) {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut rest = text;
    'scan: while !rest.is_empty() {
        for prefix in PREFIXES {
            if rest.starts_with(prefix) && !out.ends_with(is_token_char) {
                let tail = rest[prefix.len()..].chars().take_while(|c| is_token_char(*c)).map(char::len_utf8).sum::<usize>();
                if tail >= MIN_TOKEN_TAIL {
                    out.push_str(prefix);
                    out.push_str(REDACTED);
                    rest = &rest[prefix.len() + tail..];
                    changed = true;
                    continue 'scan;
                }
            }
        }
        let c = rest.chars().next()?;
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    changed.then_some(out)
}

/// The value of a credential header, to the end of the line, and a `Bearer xxx` anywhere.
fn header_values(text: &str) -> Option<String> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for header in HEADERS {
        let mut from = 0;
        while let Some(at) = find_ci(text, header, from) {
            let start = at + header.len();
            let end = text[start..].find(['\n', '\r']).map_or(text.len(), |n| start + n);
            // Keep a leading space so `Authorization: ‹redacted›` still reads as a header.
            spans.push((start + usize::from(text[start..].starts_with(' ')), end));
            from = end.max(start);
        }
    }
    let mut from = 0;
    while let Some(at) = find_ci(text, "bearer ", from) {
        let start = at + "bearer ".len();
        let end =
            text[start..].find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | ';' | ')')).map_or(text.len(), |n| start + n);
        if end > start {
            spans.push((start, end));
        }
        from = end.max(start);
    }
    replace_spans(text, spans)
}

/// `https://name:password@host/` loses `name:password`.
fn url_userinfo(text: &str) -> Option<String> {
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find("://") {
        let start = from + at + 3;
        let stop = text[start..]
            .find(|c: char| c == '/' || c.is_whitespace() || matches!(c, '"' | '\'' | '?' | '#'))
            .map_or(text.len(), |n| start + n);
        if let Some(at_sign) = text[start..stop].rfind('@') {
            spans.push((start, start + at_sign));
        }
        from = stop.max(start);
    }
    replace_spans(text, spans)
}

/// `?access_token=abc` and `&password=abc` lose the value.
fn query_values(text: &str) -> Option<String> {
    let mut spans = Vec::new();
    for param in PARAMS {
        let mut from = 0;
        while let Some(name_start) = find_ci(text, param, from) {
            let start = name_start + param.len();
            let after_separator = name_start > 0 && matches!(text.as_bytes()[name_start - 1], b'?' | b'&' | b';');
            let end = text[start..]
                .find(|c: char| c == '&' || c == ';' || c.is_whitespace() || matches!(c, '"' | '\'' | '#'))
                .map_or(text.len(), |n| start + n);
            if after_separator && end > start {
                spans.push((start, end));
            }
            from = end.max(start);
        }
    }
    replace_spans(text, spans)
}

/// Replace the byte ranges (merged where they overlap) with [`REDACTED`]. `None` when there are none.
fn replace_spans(text: &str, mut spans: Vec<(usize, usize)>) -> Option<String> {
    if spans.is_empty() {
        return None;
    }
    spans.sort_unstable();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (start, end) in spans {
        if end <= at {
            continue;
        }
        let start = start.max(at);
        out.push_str(&text[at..start]);
        out.push_str(REDACTED);
        at = end;
    }
    out.push_str(&text[at..]);
    Some(out)
}

#[cfg(test)]
mod tests;
