//! A project's README on the game page, drawn from the blocks `reclaw-media` cuts it into.
//!
//! * `view`: [`ReadmeView`], one block after another: prose to the stock markdown viewer, pictures and
//!   badges to our own components, diagrams as their source
//! * `picture`: [`FitPicture`] (a picture laid out with its true proportions, behind a placeholder)
//!   and [`BadgeChip`] (a status badge drawn natively, because the toolkit's SVG drawing has no fonts)
//! * `fetch`: [`use_readme`], the hook that fetches a README and cuts it into blocks (the section and the banner share it)
//! * `section`: [`ReadmeSection`], which shows the README, collapsed to start with
//!
//! Prose is the stock `MarkdownViewer`, restyled with Reclaw's colors; nothing of the toolkit is
//! forked or patched. Everything the stock viewer cannot do safely was already taken out of its text.
mod fetch;
mod picture;
mod section;
mod view;

pub use fetch::{Readme, readme_context, use_readme};
pub use picture::{BadgeChip, FitPicture};
pub use section::ReadmeSection;
pub use view::ReadmeView;
