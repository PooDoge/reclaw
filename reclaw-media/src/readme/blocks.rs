use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

use super::{
    Block, Document, Pic, html, rewrite,
    source::{LinkTarget, ReadmeContext},
};

/// Read a README into blocks. Only the top-level blocks of the document are cut apart, so a list,
/// a quote or a table is never split in the middle; pictures and diagrams nested in them stay in
/// the markdown (see `rewrite` for what becomes of those).
pub fn parse(source: &str, ctx: &ReadmeContext) -> Document {
    let source = source.trim_start_matches('\u{feff}');
    let parser = Parser::new_ext(source, Options::ENABLE_TABLES);
    // `[label]: address` lines produce no events, so they are collected before the walk and written
    // back into each piece that uses them.
    let definitions: Vec<(String, String)> = parser
        .reference_definitions()
        .iter()
        .filter_map(|(label, def)| match ctx.link(&def.dest) {
            LinkTarget::Web(url) => Some((label.to_string(), url)),
            LinkTarget::Anchor(_) | LinkTarget::None => None,
        })
        .collect();

    let mut events = parser.into_offset_iter();
    let mut blocks = Vec::new();
    while let Some((event, range)) = events.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                let inner = take_until_end(&mut events);
                blocks
                    .push(picture_row(source, &range, &inner, ctx).map_or_else(|| Block::Markdown(slice(source, &range)), Block::Pictures));
            }
            Event::Start(Tag::HtmlBlock) => {
                take_until_end(&mut events);
                blocks.extend(html::lower(&slice(source, &range), ctx));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let inner = take_until_end(&mut events);
                blocks.push(match diagram_language(&kind) {
                    Some(language) => Block::Diagram {
                        language,
                        source: inner.iter().filter_map(|(e, _)| if let Event::Text(t) = e { Some(t.as_ref()) } else { None }).collect(),
                    },
                    None => Block::Markdown(slice(source, &range)),
                });
            }
            Event::Start(_) => {
                take_until_end(&mut events);
                blocks.push(Block::Markdown(slice(source, &range)));
            }
            Event::Rule => blocks.push(Block::Markdown("---".to_string())),
            _ => {}
        }
    }
    finish(blocks, &definitions, ctx)
}

fn slice(source: &str, range: &Range<usize>) -> String {
    source.get(range.clone()).unwrap_or_default().trim_end().to_string()
}

/// Everything up to the end of the element just started, nested elements included.
fn take_until_end<'a>(events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>) -> Vec<(Event<'a>, Range<usize>)> {
    let mut depth = 0usize;
    let mut inner = Vec::new();
    for (event, range) in events.by_ref() {
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) if depth == 0 => break,
            Event::End(_) => depth -= 1,
            _ => {}
        }
        inner.push((event, range));
    }
    inner
}

fn diagram_language(kind: &CodeBlockKind) -> Option<String> {
    let CodeBlockKind::Fenced(info) = kind else { return None };
    let language = info.split_whitespace().next()?;
    language.eq_ignore_ascii_case("mermaid").then(|| language.to_ascii_lowercase())
}

/// The pictures of a paragraph that has nothing else in it (a logo, a row of badges, a screenshot),
/// or `None` when it has words.
fn picture_row(source: &str, span: &Range<usize>, inner: &[(Event<'_>, Range<usize>)], ctx: &ReadmeContext) -> Option<Vec<Pic>> {
    if inner.iter().any(|(e, _)| matches!(e, Event::InlineHtml(_))) {
        return html_pictures(source, span, inner, ctx);
    }
    let mut pics = Vec::new();
    let mut link: Option<String> = None;
    let mut events = inner.iter().map(|(e, _)| e);
    while let Some(event) = events.next() {
        match event {
            Event::SoftBreak | Event::HardBreak => {}
            Event::Text(t) if t.trim().is_empty() => {}
            Event::Start(Tag::Link { dest_url, link_type, .. }) if !matches!(link_type, LinkType::Autolink | LinkType::Email) => {
                link = Some(match ctx.link(dest_url) {
                    LinkTarget::Web(url) => url,
                    LinkTarget::Anchor(_) | LinkTarget::None => String::new(),
                })
                .filter(|l| !l.is_empty());
            }
            Event::End(TagEnd::Link) => link = None,
            Event::Start(Tag::Image { dest_url, .. }) => {
                let mut alt = String::new();
                let mut depth = 0usize;
                for inner_event in events.by_ref() {
                    match inner_event {
                        Event::Start(_) => depth += 1,
                        Event::End(_) if depth == 0 => break,
                        Event::End(_) => depth -= 1,
                        Event::Text(t) | Event::Code(t) => alt.push_str(t),
                        _ => {}
                    }
                }
                pics.push(Pic { link: link.clone(), ..Pic::new(ctx, dest_url, &alt) });
            }
            _ => return None,
        }
    }
    (!pics.is_empty()).then_some(pics)
}

/// A paragraph of inline HTML tags and nothing else (`<a href><img></a>`): lower it as HTML and keep
/// the result only if it is pictures.
fn html_pictures(source: &str, span: &Range<usize>, inner: &[(Event<'_>, Range<usize>)], ctx: &ReadmeContext) -> Option<Vec<Pic>> {
    let only_tags = inner.iter().all(|(e, _)| match e {
        Event::InlineHtml(_) | Event::SoftBreak | Event::HardBreak => true,
        Event::Text(t) => t.trim().is_empty(),
        _ => false,
    });
    if !only_tags {
        return None;
    }
    let lowered = html::lower(&slice(source, span), ctx);
    let mut pics = Vec::new();
    for block in lowered {
        match block {
            Block::Pictures(row) => pics.extend(row),
            _ => return None,
        }
    }
    (!pics.is_empty()).then_some(pics)
}

/// Make each piece of markdown safe, put back the definitions it uses, and drop what is empty. The
/// pieces stay as they were cut (a heading, a paragraph, a list): the UI shows the first few of them
/// until asked for the rest, and that needs them separate.
fn finish(blocks: Vec<Block>, definitions: &[(String, String)], ctx: &ReadmeContext) -> Document {
    let blocks = blocks
        .into_iter()
        .filter_map(|block| match block {
            Block::Markdown(text) => {
                let mut text = rewrite::markdown(&text, ctx);
                let lower = text.to_lowercase();
                for (label, url) in definitions {
                    if lower.contains(&format!("[{}]", label.to_lowercase())) {
                        text.push_str(&format!("\n\n[{label}]: <{url}>"));
                    }
                }
                (!text.trim().is_empty()).then_some(Block::Markdown(text))
            }
            Block::Pictures(pics) => (!pics.is_empty()).then_some(Block::Pictures(pics)),
            diagram @ Block::Diagram { .. } => Some(diagram),
        })
        .collect();
    Document { blocks }
}

#[cfg(test)]
mod tests;
