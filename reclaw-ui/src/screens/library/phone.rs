//! Phone layout: a two-column capsule grid, a pushed game page, and bottom tabs.
use freya::prelude::*;

use super::{ctx::Ctx, parts::*};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

pub(super) fn layout(c: &Ctx) -> Element {
    let t = c.t;
    let (mut nav, mut page_open) = (c.nav, c.page_open);
    let body = if page_open() {
        rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::fill())
            .padding(SPACE_4)
            .child(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Chevron)
                    .label("Library")
                    .size(ButtonSize::Touch)
                    .on_press(move |_| page_open.set(false)),
            )
            .maybe_child(c.current.clone().map(|g| hero(c, g, true)))
            .child(downloads_list(c))
            .into_element()
    } else {
        rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::fill())
            .padding(SPACE_4)
            .child(TypeStyle::Eyebrow.text("Library", t.ink_muted))
            .child(search_row(c))
            .child(chips(c))
            .child(grid(c, 2))
            .into_element()
    };
    rect()
        .vertical()
        .content(Content::Flex)
        .expanded()
        .background(t.bg_base)
        .child(ScrollView::new().show_scrollbar(false).height(Size::flex(1.)).child(body))
        .child(Nav::new(NavMode::Bottom, nav()).downloads(c.downloads.len() as u32).on_select(move |item| nav.set(item)))
        .into_element()
}
