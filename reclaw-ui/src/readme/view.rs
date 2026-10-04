use freya::{markdown::MarkdownViewerThemePartialExt, prelude::*};
use reclaw_media::readme::{Block, Document, Pic};

use super::picture::{BadgeChip, FitPicture};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// The most lines of a diagram's source shown, until diagrams are drawn.
const DIAGRAM_LINES: usize = 14;

/// A README, block after block. `blocks` is how many to show; `None` shows them all.
#[derive(Clone, PartialEq)]
pub struct ReadmeView {
    doc: Document,
    blocks: Option<usize>,
    on_open: EventHandler<String>,
}

impl ReadmeView {
    pub fn new(doc: Document, on_open: EventHandler<String>) -> Self {
        Self { doc, blocks: None, on_open }
    }

    /// Show only the first `count` blocks.
    pub fn first(mut self, count: usize) -> Self {
        self.blocks = Some(count);
        self
    }
}

impl Component for ReadmeView {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let shown = self.blocks.unwrap_or(usize::MAX);
        rect().vertical().spacing(SPACE_3).width(Size::fill()).children(self.doc.blocks.iter().take(shown).enumerate().map(|(i, block)| {
            match block {
                Block::Markdown(text) => prose(&t, text).key(format!("{i}-md")).into_element(),
                Block::Pictures(pics) => pictures(pics, &self.on_open).key(format!("{i}-pics")).into_element(),
                Block::Diagram { language, source } => diagram(&t, language, source).key(format!("{i}-diagram")).into_element(),
            }
        }))
    }
}

/// Prose, restyled with Reclaw's colors and sizes. The stock viewer does the layout.
fn prose(t: &Reclaw, text: &str) -> MarkdownViewer {
    MarkdownViewer::new(text.to_string())
        .width(Size::fill())
        .color(t.ink)
        .color_link(t.accent)
        .color_code(t.ink)
        .background_code(t.bg_raised)
        .background_blockquote(t.bg_panel)
        .border_blockquote(t.line_strong)
        .background_divider(t.line)
        .heading_h1(24.)
        .heading_h2(20.)
        .heading_h3(18.)
        .heading_h4(16.)
        .heading_h5(15.)
        .heading_h6(14.)
        .paragraph_size(14.)
        .code_font_size(13.)
        .table_font_size(13.)
}

/// Pictures on one line. Badges sit side by side at their own size; other pictures share the line
/// and each takes what it needs, up to its share.
fn pictures(pics: &[Pic], on_open: &EventHandler<String>) -> Rect {
    let all_badges = pics.iter().all(|p| p.badge.is_some());
    let items = pics.iter().enumerate().map(|(i, pic)| match &pic.badge {
        Some(badge) => BadgeChip::new(badge.clone(), pic.link.clone(), on_open.clone()).key(i).into_element(),
        None => rect().width(Size::flex(1.)).child(FitPicture::new(pic.clone(), on_open.clone()).key(i)).into_element(),
    });
    if all_badges {
        rect().horizontal().content(Content::wrap_spacing(SPACE_2)).spacing(SPACE_2).width(Size::fill()).children(items)
    } else {
        rect().horizontal().content(Content::Flex).spacing(SPACE_3).width(Size::fill()).children(items)
    }
}

/// A fenced diagram, as its source for now.
fn diagram(t: &Reclaw, language: &str, source: &str) -> Rect {
    let lines: Vec<&str> = source.lines().collect();
    let shown = lines.iter().take(DIAGRAM_LINES).copied().collect::<Vec<_>>().join("\n");
    rect()
        .vertical()
        .spacing(SPACE_1)
        .width(Size::fill())
        .padding(SPACE_3)
        .background(t.bg_raised)
        .corner_radius(RADIUS_SM)
        .child(TypeStyle::Eyebrow.text(format!("{language} diagram (source)"), t.ink_subtle))
        .child(TypeStyle::Mono.text(shown, t.ink_muted))
        .maybe(lines.len() > DIAGRAM_LINES, |el| {
            el.child(TypeStyle::Meta.text(format!("and {} more lines", lines.len() - DIAGRAM_LINES), t.ink_subtle))
        })
}
