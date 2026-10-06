//! Turning a project's README into something the UI can draw with the stock markdown viewer.
//!
//! The viewer in the UI toolkit is good at prose, lists, tables and code, and drops what GitHub
//! READMEs lean on: raw HTML (centered logos, `<details>`), images with relative paths, badges. So
//! the README is cut into [`Block`]s first. Prose stays markdown (with its relative links made
//! absolute) and goes to the viewer; pictures, badge rows and diagrams become blocks of their own
//! that the UI draws itself. Nothing here fetches or draws: it only reads text.
//!
//! * `source`: where the README lives and how its relative links and images resolve
//! * `rewrite`: making links and image paths in markdown text absolute
//! * `html`: the small subset of HTML a README uses, lowered to blocks (scripts and styles are dropped)
//! * `badge`: the status badges at the top of a README, read from their address
//! * `banner`: which of its pictures could stand in for a game's banner, best first
//! * `blocks`: the splitter that produces the [`Document`]
mod badge;
mod banner;
mod blocks;
mod html;
mod rewrite;
mod source;

pub use badge::{Badge, BadgeColor};
pub use banner::{MIN_ASPECT, MIN_WIDTH, banner_candidates, fits_banner};
pub use blocks::parse;
pub use source::{LinkTarget, ReadmeContext};

/// A picture in a README: a screenshot, a logo, a badge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pic {
    /// The address to fetch, absolute and https, or `None` when it cannot be used (a relative path
    /// that leaves the repository, a `data:` image). The UI shows the alt text instead.
    pub src: Option<String>,
    pub alt: String,
    /// Where pressing it goes, when it is wrapped in a link.
    pub link: Option<String>,
    /// The size the README asked for, in pixels.
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Set when the picture is a status badge, to be drawn natively instead of fetched.
    pub badge: Option<Badge>,
}

impl Pic {
    /// A picture from its source text and alt text, resolved against the README's home. A badge is
    /// recognized here so the UI can draw it itself.
    pub(super) fn new(ctx: &ReadmeContext, src: &str, alt: &str) -> Self {
        let src = ctx.image(src);
        let badge = src.as_deref().and_then(|url| Badge::from_image(url, alt));
        Self { src, alt: alt.trim().to_string(), link: None, width: None, height: None, badge }
    }
}

/// One piece of a README, in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// Markdown for the stock viewer, with links and image paths already absolute.
    Markdown(String),
    /// Pictures that sit on one line: a logo, a row of badges, a screenshot.
    Pictures(Vec<Pic>),
    /// A fenced diagram (`mermaid`). The UI shows the source until it can draw the diagram.
    Diagram { language: String, source: String },
}

/// A README as blocks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

/// What a picture row weighs when deciding how much of a README is "the start": about two paragraphs.
const PICTURE_WEIGHT: usize = 400;
const DIAGRAM_WEIGHT: usize = 300;

impl Document {
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// How many of the first blocks make up roughly `budget` characters of reading: the start of the
    /// README, to show before offering the rest. At least one block, and never half of one.
    pub fn preview_len(&self, budget: usize) -> usize {
        let mut used = 0;
        for (i, block) in self.blocks.iter().enumerate() {
            if used >= budget {
                return i;
            }
            used += match block {
                Block::Markdown(text) => text.len(),
                Block::Pictures(_) => PICTURE_WEIGHT,
                Block::Diagram { source, .. } => DIAGRAM_WEIGHT.max(source.len()),
            };
        }
        self.blocks.len()
    }
}
