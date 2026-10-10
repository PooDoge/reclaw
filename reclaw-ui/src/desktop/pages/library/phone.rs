//! Phone layout: a two-column capsule grid. A capsule opens the game's page.
use freya::prelude::*;

use super::{ctx::Ctx, parts::*, updates};
use crate::metrics::*;

pub(super) fn layout(c: &Ctx) -> Element {
    let t = c.t;
    rect()
        .vertical()
        .expanded()
        .background(t.bg_base)
        .child(
            ScrollView::new().show_scrollbar(false).child(
                rect()
                    .vertical()
                    .spacing(SPACE_4)
                    .width(Size::fill())
                    .padding(SPACE_4)
                    .child(top_row(c))
                    .child(chips(c))
                    .maybe_child(updates::section(c))
                    .maybe_child(search_bar(c, false))
                    .child(grid_or_nothing(c, 2)),
            ),
        )
        .into_element()
}
