//! The component behind every route. `nav::Route` maps a path to one of these; each picks the page
//! for the interface that is showing (desktop or Deck) and passes the route's parameters on.
//!
//! * `layout`: `AppLayout`, the persistent shell around the page
//! * `routes`: one component per route, in the order of the route table
mod layout;
mod routes;

pub use layout::AppLayout;
pub use routes::*;
