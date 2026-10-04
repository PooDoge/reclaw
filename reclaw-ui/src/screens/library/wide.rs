//! Wide layout: top nav, a sidebar of rows, the hero and downloads in the main column.
use freya::prelude::*;

use super::{ctx::Ctx, parts::*};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

pub(super) fn layout(c: &Ctx) -> Element {
    let t = c.t;
    let (mut selected, mut nav) = (c.selected, c.nav);
    let rows = c.visible.iter().cloned().map(|g| {
        let id = g.id;
        LibraryRow::new(g)
            .selected(Some(id) == selected())
            .density(c.density)
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
        .child(rect().padding(Gaps::new(SPACE_2, 0., 0., SPACE_1)).child(TypeStyle::Eyebrow.text("Library", t.ink_subtle)))
        .child(ScrollView::new().show_scrollbar(false).height(Size::flex(1.)).child(rect().vertical().spacing(2.).children(rows)));
    let main = rect().vertical().spacing(SPACE_5).width(Size::flex(1.)).height(Size::fill()).padding(SPACE_5).child(
        ScrollView::new().show_scrollbar(false).child(
            rect()
                .vertical()
                .spacing(SPACE_5)
                .width(Size::fill())
                .maybe_child(c.current.clone().map(|g| hero(c, g, false)))
                .child(downloads_list(c)),
        ),
    );
    let status = rect()
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .width(Size::fill())
        .height(Size::px(28.))
        .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
        .background(t.bg_deep)
        .child(rect().width(Size::flex(1.)).child(TypeStyle::Meta.text("Library synced", t.ink_subtle)))
        .child(
            TypeStyle::Mono
                .text(format!("{} apps  {} {}", c.games.len(), c.updates, if c.updates == 1 { "update" } else { "updates" }), t.ink_subtle),
        );

    // The 40px top bar always takes the pointer-height field, even at touch density.
    let mut topbar = Nav::new(NavMode::Top, nav()).downloads(c.downloads.len() as u32);
    if let Some(button) = deck_button(c) {
        topbar = topbar.actions(button);
    }
    let topbar = topbar.trailing(SearchField::new(c.search)).on_select(move |item| nav.set(item));

    rect()
        .vertical()
        .content(Content::Flex)
        .expanded()
        .background(t.bg_base)
        .child(topbar)
        .child(rect().horizontal().content(Content::Flex).width(Size::fill()).height(Size::flex(1.)).child(sidebar).child(main))
        .child(status)
        .into_element()
}
