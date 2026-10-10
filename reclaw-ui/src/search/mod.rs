//! The search button in the desktop's top bar (or floating at the top right on a narrow window) and what it searches: the tab
//! that is showing. Pure: which tab a search belongs to, the text being typed and the search that is running for each tab, how
//! a query matches and which parts of a line to highlight, and which settings rows a query finds. `desktop::search` draws it.
//!
//! * `scope`: which tab a search is for, its words, the page its results show on
//! * `state`: open or closed, what is typed, what was searched, and how long a half-typed search is kept
//! * `text`: words of a query, matching, highlighting
//! * `settings`: a settings schema narrowed to the rows a query finds
mod scope;
mod settings;
mod state;
mod text;

pub use scope::SearchScope;
pub use settings::{filter_schema, row_count};
pub use state::{DRAFT_KEPT_SECS, ScopeSearch, SearchModel};
pub use text::{Piece, highlight, matches_all, results_line, terms};
