//! Phone layout: a two-column capsule grid. A capsule opens the game's page.
use freya::prelude::*;

use super::{ctx::Ctx, parts::*};
use crate::{metrics::*, typography::TypeStyle};

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
                    .child(TypeStyle::Eyebrow.text("Library", t.ink_muted))
                    .child(search_row(c))
                    .child(chips(c))
                    .child(grid(c, 2)),
            ),
        )
        .into_element()
}
