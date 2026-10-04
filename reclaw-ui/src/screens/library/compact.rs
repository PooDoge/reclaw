//! Compact layout: icon rail, search, chips, the hero and a four-column capsule grid.
use freya::prelude::*;

use super::{ctx::Ctx, parts::*};
use crate::{metrics::*, prelude::*};

pub(super) fn layout(c: &Ctx) -> Element {
    let t = c.t;
    let mut nav = c.nav;
    rect()
        .horizontal()
        .content(Content::Flex)
        .expanded()
        .background(t.bg_base)
        .child(Nav::new(NavMode::Rail, nav()).downloads(c.downloads.len() as u32).on_select(move |item| nav.set(item)))
        .child(
            rect()
                .vertical()
                .content(Content::Flex)
                .width(Size::flex(1.))
                .height(Size::fill())
                .child(rect().width(Size::fill()).padding(Gaps::new(SPACE_3, SPACE_4, SPACE_3, SPACE_4)).child(search_row(c)))
                .child(
                    ScrollView::new().show_scrollbar(false).height(Size::flex(1.)).child(
                        rect()
                            .vertical()
                            .spacing(SPACE_4)
                            .width(Size::fill())
                            .padding(SPACE_4)
                            .child(chips(c))
                            .maybe_child(c.current.clone().map(|g| hero(c, g, true)))
                            .child(grid(c, 4)),
                    ),
                ),
        )
        .into_element()
}
