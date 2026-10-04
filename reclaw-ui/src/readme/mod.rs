//! A project's README on the game page, drawn from the blocks `reclaw-media` cuts it into.
//!
//! * `view`: [`ReadmeView`], one block after another: prose to the stock markdown viewer, pictures and
//!   badges to our own components, diagrams as their source
//! * `picture`: [`FitPicture`] (a picture laid out with its true proportions, behind a placeholder)
//!   and [`BadgeChip`] (a status badge drawn natively, because the toolkit's SVG drawing has no fonts)
//! * `section`: [`ReadmeSection`], which fetches the README and shows it, collapsed to start with
//!
//! Prose is the stock `MarkdownViewer`, restyled with Reclaw's colors; nothing of the toolkit is
//! forked or patched. Everything the stock viewer cannot do safely was already taken out of its text.
mod picture;
mod section;
mod view;

pub use picture::{BadgeChip, FitPicture};
pub use section::ReadmeSection;
pub use view::ReadmeView;
