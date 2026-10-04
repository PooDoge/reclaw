//! The small part of HTML that READMEs use, turned into [`Block`]s: centered logos, badge rows,
//! headings, `<details>`, lists, `<pre>`. Everything else is dropped, and what is dropped on purpose
//! (scripts, styles, frames, forms, inline SVG) is dropped with its contents.
//!
//! This is not an HTML engine. It reads tags and text, keeps what the markdown viewer can show, and
//! never follows, runs or fetches anything: pictures become [`Pic`]s for the UI to fetch under its own rules.
use super::{
    Block, Pic,
    source::{LinkTarget, ReadmeContext},
};

mod tokens;

use tokens::{Token, tokenize};

/// Elements whose whole subtree is dropped.
const SKIPPED: &[&str] = &[
    "script", "style", "iframe", "object", "embed", "form", "button", "select", "textarea", "template", "svg", "canvas", "video", "audio",
    "noscript", "head", "title",
];
/// Elements that start a new paragraph (and end the one before).
const BLOCKS: &[&str] = &[
    "p",
    "div",
    "section",
    "article",
    "header",
    "footer",
    "main",
    "nav",
    "aside",
    "center",
    "figure",
    "figcaption",
    "table",
    "tr",
    "blockquote",
    "dl",
    "dt",
    "dd",
    "address",
    "tbody",
    "thead",
];

#[derive(Clone, Copy)]
enum List {
    Bullet,
    Numbered,
}

struct Anchor {
    href: String,
    text_start: usize,
    pics_start: usize,
}

struct Lower<'a> {
    ctx: &'a ReadmeContext,
    blocks: Vec<Block>,
    /// The paragraph being built, as markdown.
    text: String,
    /// What goes in front of it when it ends: `## ` for a heading, `- ` for a list item.
    prefix: String,
    /// What wraps it when it ends (`**` for a `<summary>`).
    wrap: &'static str,
    /// Pictures on the line being built.
    pics: Vec<Pic>,
    anchors: Vec<Anchor>,
    lists: Vec<List>,
    skip: Option<(String, usize)>,
    pre: Option<String>,
    in_code: usize,
}

/// Lower an HTML fragment. `ctx` resolves the addresses of pictures and links.
pub(super) fn lower(html: &str, ctx: &ReadmeContext) -> Vec<Block> {
    let mut lower = Lower {
        ctx,
        blocks: Vec::new(),
        text: String::new(),
        prefix: String::new(),
        wrap: "",
        pics: Vec::new(),
        anchors: Vec::new(),
        lists: Vec::new(),
        skip: None,
        pre: None,
        in_code: 0,
    };
    for token in tokenize(html) {
        lower.token(token);
    }
    lower.end_paragraph();
    lower.blocks
}

fn is_void(name: &str) -> bool {
    matches!(name, "br" | "hr" | "img" | "input" | "meta" | "link" | "source" | "wbr" | "area" | "base" | "col" | "param" | "track")
}

impl Lower<'_> {
    fn token(&mut self, token: Token) {
        if let Some((tag, depth)) = &mut self.skip {
            match &token {
                Token::Open { name, self_closing: false, .. } if name == tag => *depth += 1,
                Token::Close(name) if name == tag => {
                    *depth -= 1;
                    if *depth == 0 {
                        self.skip = None;
                    }
                }
                _ => {}
            }
            return;
        }
        match token {
            Token::Text(text) => self.text(&text),
            Token::Open { name, attrs, self_closing } => self.open(&name, &attrs, self_closing),
            Token::Close(name) => self.close(&name),
        }
    }

    fn open(&mut self, name: &str, attrs: &[(String, String)], self_closing: bool) {
        let attr = |key: &str| attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
        if SKIPPED.contains(&name) {
            if !self_closing && !is_void(name) {
                self.skip = Some((name.to_string(), 1));
            }
            return;
        }
        match name {
            "img" => self.image(attr("src").unwrap_or_default(), attr("alt").unwrap_or_default(), attr("width"), attr("height")),
            "br" => self.text.push_str("  \n"),
            "hr" => {
                self.end_paragraph();
                self.blocks.push(Block::Markdown("---".to_string()));
            }
            "a" => self.anchors.push(Anchor {
                href: attr("href").unwrap_or_default().to_string(),
                text_start: self.text.len(),
                pics_start: self.pics.len(),
            }),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.end_paragraph();
                let level = name.as_bytes()[1] - b'0';
                self.prefix = format!("{} ", "#".repeat(usize::from(level)));
            }
            "ul" | "ol" => {
                self.end_paragraph();
                self.lists.push(if name == "ol" { List::Numbered } else { List::Bullet });
            }
            "li" => {
                self.end_paragraph();
                let depth = self.lists.len().saturating_sub(1);
                let marker = if matches!(self.lists.last(), Some(List::Numbered)) { "1. " } else { "- " };
                self.prefix = format!("{}{marker}", "  ".repeat(depth));
            }
            "summary" => {
                self.end_paragraph();
                self.wrap = "**";
            }
            "pre" => {
                self.end_paragraph();
                self.pre = Some(String::new());
            }
            "code" | "kbd" | "tt" if self.pre.is_none() => {
                self.text.push('`');
                self.in_code += 1;
            }
            "b" | "strong" => self.text.push_str("**"),
            "i" | "em" => self.text.push('*'),
            _ if BLOCKS.contains(&name) => self.end_paragraph(),
            // Everything else (span, font, small, picture, details, sup, sub, u, mark ...) just lets its content through.
            _ => {}
        }
    }

    fn close(&mut self, name: &str) {
        match name {
            "a" => self.close_anchor(),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "summary" => self.end_paragraph(),
            "ul" | "ol" => {
                self.end_paragraph();
                self.lists.pop();
            }
            "pre" => self.end_pre(),
            "code" | "kbd" | "tt" if self.pre.is_none() && self.in_code > 0 => {
                self.in_code -= 1;
                self.text.push('`');
            }
            "b" | "strong" => self.text.push_str("**"),
            "i" | "em" => self.text.push('*'),
            "details" => self.end_paragraph(),
            _ if BLOCKS.contains(&name) => self.end_paragraph(),
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if let Some(pre) = &mut self.pre {
            pre.push_str(text);
            return;
        }
        if text.trim().is_empty() {
            // Space between inline things is kept as one space; at the start of a line it is nothing.
            if !self.text.is_empty() && !self.text.ends_with([' ', '\n']) {
                self.text.push(' ');
            }
            return;
        }
        // Words after pictures start a new line: the pictures are a row of their own.
        self.end_pics();
        let collapsed = collapse_spaces(text, self.text.ends_with([' ', '\n']) || self.text.is_empty());
        if self.in_code > 0 { self.text.push_str(&collapsed) } else { self.text.push_str(&escape(&collapsed, self.text.is_empty())) }
    }

    fn image(&mut self, src: &str, alt: &str, width: Option<&str>, height: Option<&str>) {
        self.end_text();
        self.pics.push(Pic { width: pixels(width), height: pixels(height), ..Pic::new(self.ctx, src, alt) });
    }

    fn close_anchor(&mut self) {
        let Some(anchor) = self.anchors.pop() else { return };
        let link = match self.ctx.link(&anchor.href) {
            LinkTarget::Web(url) => Some(url),
            LinkTarget::Anchor(_) | LinkTarget::None => None,
        };
        // Pictures inside the link are pressable.
        for pic in &mut self.pics[anchor.pics_start..] {
            pic.link = pic.link.take().or_else(|| link.clone());
        }
        // Words inside it become a markdown link; the address is made absolute with the rest, later.
        let simple = !anchor.href.is_empty() && !anchor.href.contains(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | ')'));
        if simple && link.is_some() && self.text.len() > anchor.text_start && self.pics.len() == anchor.pics_start {
            let words = self.text.split_off(anchor.text_start);
            if words.trim().is_empty() {
                self.text.push_str(&words);
            } else {
                self.text.push_str(&format!("[{}](<{}>)", words.trim(), anchor.href));
            }
        }
    }

    /// The words so far become a paragraph; pictures before them are already a row.
    fn end_text(&mut self) {
        let words = std::mem::take(&mut self.text);
        let prefix = std::mem::take(&mut self.prefix);
        let wrap = std::mem::take(&mut self.wrap);
        let words = words.trim();
        if !words.is_empty() {
            self.end_pics();
            self.blocks.push(Block::Markdown(format!("{prefix}{wrap}{words}{wrap}")));
        }
    }

    fn end_pics(&mut self) {
        if !self.pics.is_empty() {
            self.blocks.push(Block::Pictures(std::mem::take(&mut self.pics)));
        }
    }

    fn end_paragraph(&mut self) {
        self.end_text();
        self.end_pics();
    }

    fn end_pre(&mut self) {
        let Some(code) = self.pre.take() else { return };
        let code = code.trim_matches('\n');
        if code.trim().is_empty() {
            return;
        }
        // A fence longer than any run of backticks inside, so the code cannot close it early.
        let longest = code.split(|c| c != '`').map(str::len).max().unwrap_or(0);
        let fence = "`".repeat((longest + 1).max(3));
        self.blocks.push(Block::Markdown(format!("{fence}\n{code}\n{fence}")));
    }
}

/// `200`, `200px` and `200.5` are widths; `50%` and `auto` are not something to hold a picture to.
fn pixels(value: Option<&str>) -> Option<u32> {
    let text = value?.trim().trim_end_matches("px");
    let number: f32 = text.parse().ok()?;
    (1. ..=10_000.).contains(&number).then_some(number as u32)
}

/// Runs of white space are one space, as HTML shows them; `leading` drops one at the start.
fn collapse_spaces(text: &str, leading: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = leading;
    for c in text.chars() {
        if c.is_whitespace() {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    out
}

/// Characters that would turn words into markdown: stray `*` or `_` pairs, brackets, backticks, and
/// the marks that begin a heading, list item or quote at the start of a line.
fn escape(text: &str, line_start: bool) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    for (i, c) in text.chars().enumerate() {
        let starts_block = i == 0 && line_start && matches!(c, '#' | '-' | '+' | '>' | '=');
        if matches!(c, '*' | '_' | '`' | '[' | ']' | '<' | '\\' | '|') || starts_block {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests;
