//! Cutting HTML into tags and text. Forgiving by design: whatever it cannot read as a tag is text.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    Open { name: String, attrs: Vec<(String, String)>, self_closing: bool },
    Close(String),
    Text(String),
}

/// Elements whose content is not markup: it ends only at the matching close tag.
const RAW_TEXT: &[&str] = &["script", "style", "textarea", "title"];

pub(super) fn tokenize(html: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut text_start = 0;
    let mut i = 0;
    while i < html.len() {
        if !html[i..].starts_with('<') {
            i += html[i..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        let rest = &html[i..];
        let skip_to = if rest.starts_with("<!--") {
            Some(rest.find("-->").map_or(rest.len(), |end| end + 3))
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            Some(rest.find('>').map_or(rest.len(), |end| end + 1))
        } else {
            None
        };
        if let Some(len) = skip_to {
            flush(&mut tokens, &html[text_start..i]);
            i += len;
            text_start = i;
            continue;
        }
        match read_tag(rest) {
            Some((token, len)) => {
                flush(&mut tokens, &html[text_start..i]);
                i += len;
                let raw = match &token {
                    Token::Open { name, self_closing: false, .. } if RAW_TEXT.contains(&name.as_str()) => Some(name.clone()),
                    _ => None,
                };
                tokens.push(token);
                if let Some(name) = raw {
                    // Skip to the close tag without reading anything in between as markup.
                    let end = find_close(&html[i..], &name).unwrap_or(html.len() - i);
                    i += end;
                }
                text_start = i;
            }
            // A lone `<` ("a < b") is just a character.
            None => i += 1,
        }
    }
    flush(&mut tokens, &html[text_start..]);
    tokens
}

fn flush(tokens: &mut Vec<Token>, text: &str) {
    if !text.is_empty() {
        tokens.push(Token::Text(decode_entities(text)));
    }
}

fn find_close(rest: &str, name: &str) -> Option<usize> {
    let lower = rest.to_ascii_lowercase();
    lower.find(&format!("</{name}"))
}

/// Read a tag at the start of `rest`; its token and how many bytes it took.
fn read_tag(rest: &str) -> Option<(Token, usize)> {
    let bytes = rest.as_bytes();
    let closing = bytes.get(1) == Some(&b'/');
    let mut i = 1 + usize::from(closing);
    let name_start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'-' | b':')) {
        i += 1;
    }
    if i == name_start || !bytes[name_start].is_ascii_alphabetic() {
        return None;
    }
    let name = rest[name_start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        match bytes.get(i)? {
            b'>' => {
                i += 1;
                break;
            }
            b'/' => {
                self_closing = true;
                i += 1;
            }
            _ => {
                let key_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && !matches!(bytes[i], b'=' | b'>' | b'/') {
                    i += 1;
                }
                let key = rest[key_start..i].to_ascii_lowercase();
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                let mut value = String::new();
                if bytes.get(i) == Some(&b'=') {
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    match bytes.get(i)? {
                        quote @ (b'"' | b'\'') => {
                            let start = i + 1;
                            let end = rest[start..].find(*quote as char)? + start;
                            value = decode_entities(&rest[start..end]);
                            i = end + 1;
                        }
                        _ => {
                            let start = i;
                            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                                i += 1;
                            }
                            value = decode_entities(&rest[start..i]);
                        }
                    }
                }
                if !key.is_empty() {
                    attrs.push((key, value));
                }
            }
        }
    }
    let token = if closing { Token::Close(name) } else { Token::Open { name, attrs, self_closing } };
    Some((token, i))
}

/// The named entities that appear in READMEs, and numeric ones. Anything else is left as written.
pub(super) fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest.find(';').filter(|end| *end <= 10).and_then(|end| entity(&rest[1..end]).map(|c| (c, end + 1)));
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        "copy" => Some('©'),
        "hellip" => Some('…'),
        "mdash" => Some('—'),
        "ndash" => Some('–'),
        number => {
            let code = number.strip_prefix('#')?;
            let value = match code.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => code.parse().ok()?,
            };
            char::from_u32(value).filter(|c| !c.is_control())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(name: &str, attrs: &[(&str, &str)]) -> Token {
        Token::Open { name: name.into(), attrs: attrs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), self_closing: false }
    }

    #[test]
    fn tags_attributes_and_text() {
        let tokens = tokenize("<p align=\"center\">Hello <b>there</b></p>");
        assert_eq!(
            tokens,
            [
                open("p", &[("align", "center")]),
                Token::Text("Hello ".into()),
                open("b", &[]),
                Token::Text("there".into()),
                Token::Close("b".into()),
                Token::Close("p".into())
            ]
        );
    }

    #[test]
    fn attribute_styles_and_case() {
        let tokens = tokenize("<IMG SRC='a b.png' width=200 alt=\"x &amp; y\" hidden/>");
        let Token::Open { name, attrs, self_closing } = &tokens[0] else { panic!("{tokens:?}") };
        assert_eq!(name, "img");
        assert!(*self_closing);
        assert_eq!(
            attrs,
            &[
                ("src".into(), "a b.png".into()),
                ("width".into(), "200".into()),
                ("alt".into(), "x & y".into()),
                ("hidden".into(), String::new())
            ]
        );
    }

    #[test]
    fn comments_and_doctype_vanish() {
        assert_eq!(
            tokenize("a<!-- hidden <b> -->b<!DOCTYPE html>c"),
            [Token::Text("a".into()), Token::Text("b".into()), Token::Text("c".into())]
        );
    }

    #[test]
    fn script_content_is_not_read_as_markup() {
        let tokens = tokenize("<script>if (a < b) { x = \"</p>\"; }</script><p>after</p>");
        assert_eq!(tokens[0], open("script", &[]));
        assert_eq!(tokens[1], Token::Close("script".into()));
        assert_eq!(tokens[2], open("p", &[]));
    }

    #[test]
    fn a_lone_less_than_is_text_and_an_unfinished_tag_is_not_a_crash() {
        assert_eq!(tokenize("1 < 2"), [Token::Text("1 < 2".into())]);
        for broken in ["<", "<a", "<a href=", "<a href=\"x", "<p align", "</", "<!--", "<img src=\"a.png\" "] {
            let _ = tokenize(broken);
        }
    }

    #[test]
    fn entities_named_and_numeric() {
        assert_eq!(decode_entities("a &amp; b &lt;c&gt; &quot;d&quot; &#65;&#x42; &nbsp;e &bogus; &"), "a & b <c> \"d\" AB  e &bogus; &");
        assert_eq!(decode_entities("&#0;"), "&#0;", "a control character is not decoded");
    }
}
