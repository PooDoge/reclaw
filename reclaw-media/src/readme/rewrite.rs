//! Making a markdown segment safe and complete for the stock viewer.
//!
//! * Link destinations become absolute, so the viewer opens them in the browser instead of treating
//!   `docs/INSTALL.md` as a page of this app. Links that cannot be opened safely (`javascript:`,
//!   `mailto:`, `#section`) become plain text: the viewer would try to open them anyway.
//! * Images in the middle of prose become links to the image. The viewer fetches images itself,
//!   with none of the size, address and cache rules of this crate, so no image is left for it.
//!   (Pictures on their own line never get here: `blocks` lifts them out and the UI draws them.)
//! * Raw HTML inside a paragraph is dropped, except a line break.
//!
//! Edits are made on the source text by position, not by re-writing the document, so everything
//! else in it stays exactly as the author typed it.
use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

use super::source::{LinkTarget, ReadmeContext};

struct Edit {
    range: Range<usize>,
    text: String,
}

pub(super) fn markdown(source: &str, ctx: &ReadmeContext) -> String {
    let mut edits: Vec<Edit> = Vec::new();
    let mut image_depth = 0usize;
    let mut link_depth = 0usize;
    let mut alt = String::new();
    // What the outermost image looks like, saved when it starts: its source span and destination.
    let mut image: Option<(Range<usize>, String)> = None;

    for (event, range) in Parser::new_ext(source, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                if image_depth == 0 {
                    alt.clear();
                    image = Some((range, dest_url.to_string()));
                }
                image_depth += 1;
            }
            Event::End(TagEnd::Image) => {
                image_depth = image_depth.saturating_sub(1);
                if image_depth == 0
                    && let Some((span, dest)) = image.take()
                {
                    edits.push(Edit { range: span, text: image_text(&alt, &dest, link_depth > 0, ctx) });
                }
            }
            Event::Text(text) | Event::Code(text) if image_depth > 0 => alt.push_str(&text),
            _ if image_depth > 0 => {}
            Event::Start(Tag::Link { link_type, dest_url, .. }) => {
                link_depth += 1;
                if !matches!(link_type, LinkType::Autolink | LinkType::Email)
                    && let Some(edit) = link_edit(source, range, link_type, &dest_url, ctx)
                {
                    edits.push(edit);
                }
            }
            Event::End(TagEnd::Link) => link_depth = link_depth.saturating_sub(1),
            Event::InlineHtml(html) => {
                edits.push(Edit { range, text: if is_line_break(&html) { "  \n".to_string() } else { String::new() } })
            }
            _ => {}
        }
    }
    apply(source, edits)
}

/// An image in prose, as text: a link to it, or only its words when it is already inside a link.
fn image_text(alt: &str, dest: &str, inside_link: bool, ctx: &ReadmeContext) -> String {
    let words = if alt.trim().is_empty() { "image" } else { alt.trim() };
    match ctx.image(dest) {
        Some(url) if !inside_link => format!("[{}]({url})", escape_label(words)),
        _ => escape_label(words),
    }
}

/// How to change a link: its destination made absolute, or the link replaced by its words.
fn link_edit(source: &str, span: Range<usize>, kind: LinkType, dest: &str, ctx: &ReadmeContext) -> Option<Edit> {
    let text = source.get(span.clone())?;
    // Only inline links, `[words](destination)`, are changed in place. The other kinds point at a
    // definition elsewhere; those are handled by `blocks`, which writes the definitions out absolute.
    if kind != LinkType::Inline {
        return None;
    }
    let close = text.rfind("](")?;
    let words = text.get(1..close)?;
    let after = &text[close + 2..];
    let lead = after.len() - after.trim_start().len();
    let angled = after[lead..].starts_with('<');
    let dest_start = span.start + close + 2 + lead + usize::from(angled);
    if !source.get(dest_start..)?.starts_with(dest) {
        return None;
    }
    // An address that is already complete is left exactly as the author wrote it.
    if dest.starts_with("https://") || dest.starts_with("http://") {
        return None;
    }
    match ctx.link(dest) {
        LinkTarget::Web(url) => Some(Edit { range: dest_start..dest_start + dest.len(), text: url }),
        LinkTarget::Anchor(_) | LinkTarget::None => Some(Edit { range: span, text: words.to_string() }),
    }
}

fn is_line_break(html: &str) -> bool {
    let tag = html.trim().trim_start_matches('<').trim_end_matches('>').trim_end_matches('/').trim();
    tag.eq_ignore_ascii_case("br")
}

fn escape_label(text: &str) -> String {
    text.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]")
}

/// Apply edits from the end, so earlier positions stay valid. An edit inside another (a link
/// destination edit within an image that was rewritten) is skipped in favor of the outer one.
fn apply(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|e| (e.range.start, std::cmp::Reverse(e.range.end)));
    let mut kept: Vec<Edit> = Vec::new();
    for edit in edits {
        if kept.last().is_none_or(|last| edit.range.start >= last.range.end) {
            kept.push(edit);
        }
    }
    let mut out = source.to_string();
    for edit in kept.into_iter().rev() {
        out.replace_range(edit.range, &edit.text);
    }
    out
}

#[cfg(test)]
mod tests;
