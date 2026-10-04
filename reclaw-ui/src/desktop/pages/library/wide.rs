//! Wide layout: a sidebar of rows, and the hero and downloads in the main column.
use freya::prelude::*;

use super::{ctx::Ctx, parts::*, updates};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

pub(super) fn layout(c: &Ctx) -> Element {
    let t = c.t;
    let mut selected = c.selected;
    let rows = c.visible.iter().cloned().map(|g| {
        let id = g.id;
        LibraryRow::new(g)
            .selected(Some(id) == c.current.as_ref().map(|g| g.id))
            .density(c.env.density)
            .on_press(move |_| selected.set(Some(id)))
            .key(id)
            .into_element()
    });
    let sidebar = rect()
        .vertical()
        .spacing(SPACE_2)
        .width(Size::px(SIDEBAR_W))
        .height(Size::fill())
        .padding(SPACE_3)
        .background(t.bg_base)
        .border(Border::new().fill(t.line).width(BorderWidth { right: 1., ..Default::default() }))
        .child(chips(c))
        .maybe_child(updates::section(c))
        .child(rect().padding(Gaps::new(SPACE_2, 0., 0., SPACE_1)).child(TypeStyle::Eyebrow.text("Library", t.ink_subtle)))
        .child(ScrollView::new().show_scrollbar(false).height(Size::flex(1.)).child(rect().vertical().spacing(2.).children(rows)));
    let main = rect().vertical().spacing(SPACE_5).width(Size::flex(1.)).height(Size::fill()).padding(SPACE_5).child(
        ScrollView::new().show_scrollbar(false).child(
            rect()
                .vertical()
                .spacing(SPACE_5)
                .width(Size::fill())
                .maybe_child(c.current.clone().map(|g| hero(c, g, false)))
                .maybe_child(downloads_list(c)),
        ),
    );
    rect().horizontal().content(Content::Flex).expanded().background(t.bg_base).child(sidebar).child(main).into_element()
}
