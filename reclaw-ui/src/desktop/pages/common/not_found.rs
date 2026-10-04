use freya::prelude::*;

use super::PageHeader;
use crate::{
    metrics::*,
    nav::{Route, use_nav},
    prelude::*,
    typography::TypeStyle,
};

/// What a page shows for an id nothing knows: a stale link, a game removed from the catalog. Says so
/// and offers the way out, rather than a blank window.
#[derive(PartialEq)]
pub struct NotFound {
    /// "game", "mod": what was looked for.
    pub what: &'static str,
    pub id: String,
}

impl Component for NotFound {
    fn render(&self) -> impl IntoElement {
        let (t, nav) = (use_reclaw(), use_nav());
        rect()
            .vertical()
            .spacing(SPACE_4)
            .expanded()
            .padding(SPACE_5)
            .child(PageHeader::new(format!("No such {}", self.what)))
            .child(TypeStyle::Body.text(
                format!("Reclaw does not know a {} called {:?}. It may have been removed from the catalog.", self.what, self.id),
                t.ink_muted,
            ))
            .child(
                rect()
                    .horizontal()
                    .child(ActionButton::new(ButtonVariant::Primary).label("Go to Library").on_press(move |_| nav.open(Route::Library {}))),
            )
    }
}
