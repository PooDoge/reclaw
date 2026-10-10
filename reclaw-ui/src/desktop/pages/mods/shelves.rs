//! The Mods tab's body: the shelves, one shelf opened page by page, and a search's results. Functions, not components: the page
//! reads the hooks once and hands them down (AGENTS rule 5).
use freya::prelude::*;

use crate::{
    desktop::{
        DesktopUi,
        pages::common::{ModRow, no_results, results_bar},
    },
    metrics::*,
    mod_shelves::{OpenShelf, PREVIEW, Paging, Shelf, search, shelf, shelves},
    prelude::*,
    search::SearchScope,
    typography::TypeStyle,
};

fn shelf_icon(shelf: Shelf) -> IconName {
    match shelf {
        Shelf::Installed => IconName::Check,
        Shelf::Popular => IconName::Download,
        Shelf::TopRated => IconName::Star,
        Shelf::Updated => IconName::Refresh,
        Shelf::Newest => IconName::Clock,
    }
}

/// Mods in rows of `columns`, each row as tall as its tallest mod.
fn grid(mods: &[ModEntry], columns: usize, query: &str) -> Rect {
    let columns = columns.max(1);
    rect().vertical().spacing(SPACE_2).width(Size::fill()).children(mods.chunks(columns).enumerate().map(|(row, chunk)| {
        let mut line = rect().horizontal().content(Content::Flex).spacing(SPACE_2).width(Size::fill());
        for m in chunk {
            let key = format!("{}/{}", m.provider.slug(), m.id);
            line = line.child(rect().width(Size::flex(1.)).child(ModRow::new(m.clone()).query(query).key(key)));
        }
        for _ in chunk.len()..columns {
            line = line.child(rect().width(Size::flex(1.)));
        }
        line.key(row).into_element()
    }))
}

/// A shelf's title line: its icon and name, how many are on it, what the order is, and what its button does.
fn shelf_header(t: &Reclaw, shelf: Shelf, count: usize, action: Option<Element>) -> Rect {
    rect()
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(SPACE_3)
        .width(Size::fill())
        .child(rect().width(Size::px(32.)).height(Size::px(32.)).center().corner_radius(RADIUS_MD).background(t.bg_raised).child(icon(
            shelf_icon(shelf),
            16.,
            t.accent,
        )))
        .child(
            rect()
                .vertical()
                .width(Size::flex(1.))
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(SPACE_2)
                        .child(TypeStyle::Heading.text(shelf.title(), t.ink))
                        .child(TypeStyle::Mono.text(count.to_string(), t.ink_subtle)),
                )
                .child(TypeStyle::Meta.text(shelf.note(), t.ink_subtle)),
        )
        .maybe_child(action)
}

/// Every shelf with something on it, a few mods each, and a way to open the rest.
pub(super) fn overview(t: &Reclaw, ui: DesktopUi, mods: &[ModEntry], columns: usize) -> Element {
    let mut open = ui.mod_shelf;
    // Two columns show two rows of a shelf, one column four mods.
    let preview = if columns > 1 { PREVIEW } else { PREVIEW.min(3) };
    rect()
        .vertical()
        .spacing(SPACE_6)
        .width(Size::fill())
        .children(shelves(mods).into_iter().map(|(which, on)| {
            let more = (on.len() > preview).then(|| {
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Chevron)
                    .label(format!("Show all {}", on.len()))
                    .on_press(move |_| open.set(Some(OpenShelf { shelf: which, page: 0 })))
                    .into_element()
            });
            rect()
                .vertical()
                .spacing(SPACE_3)
                .width(Size::fill())
                .child(shelf_header(t, which, on.len(), more))
                .child(grid(&on[..on.len().min(preview)], columns, ""))
                .key(which.title())
                .into_element()
        }))
        .into_element()
}

/// The pager under a long list: back, where in the list this page is, forward. Nothing when it all fits on one page.
fn pager(t: &Reclaw, paging: Paging, on_page: EventHandler<usize>) -> Option<Rect> {
    (paging.pages > 1).then(|| {
        let (previous, next) = (on_page.clone(), on_page);
        rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(
                ActionButton::new(ButtonVariant::Secondary)
                    .icon(IconName::Back)
                    .label("Previous")
                    .enabled(paging.has_previous())
                    .on_press(move |_| previous.call(paging.page.saturating_sub(1))),
            )
            .child(rect().width(Size::flex(1.)).center().child(
                TypeStyle::Meta.text(format!("Page {} of {}  \u{b7}  {}", paging.page + 1, paging.pages, paging.label()), t.ink_muted),
            ))
            .child(
                ActionButton::new(ButtonVariant::Secondary)
                    .icon(IconName::Chevron)
                    .label("Next")
                    .enabled(paging.has_next())
                    .on_press(move |_| next.call(paging.page + 1)),
            )
    })
}

/// One shelf, whole, a page at a time.
pub(super) fn opened(t: &Reclaw, ui: DesktopUi, mods: &[ModEntry], open: OpenShelf, columns: usize) -> Element {
    let mut state = ui.mod_shelf;
    let on = shelf(mods, open.shelf);
    let paging = Paging::of(on.len(), open.page);
    let back =
        ActionButton::new(ButtonVariant::Ghost).icon(IconName::Back).label("All shelves").on_press(move |_| state.set(None)).into_element();
    let on_page = EventHandler::new(move |page: usize| state.set(Some(OpenShelf { page, ..open })));
    rect()
        .vertical()
        .spacing(SPACE_3)
        .width(Size::fill())
        .child(shelf_header(t, open.shelf, on.len(), Some(back)))
        .child(grid(&on[paging.start..paging.end], columns, ""))
        .maybe_child(pager(t, paging, on_page))
        .into_element()
}

/// A search's results within what the chips leave, a page at a time, the words that matched highlighted.
pub(super) fn results(t: &Reclaw, ui: DesktopUi, mods: &[ModEntry], query: &str, columns: usize) -> Element {
    let found = search(mods, query);
    let on_clear = EventHandler::new(move |()| ui.clear_search(SearchScope::Mods));
    if found.is_empty() {
        return no_results(t, SearchScope::Mods, query, "Try other words, another game or All sites.", on_clear).into_element();
    }
    let mut page = ui.mod_results_page;
    let paging = Paging::of(found.len(), *page.read());
    rect()
        .vertical()
        .spacing(SPACE_3)
        .width(Size::fill())
        .child(results_bar(t, SearchScope::Mods, query, found.len(), false, on_clear))
        .child(grid(&found[paging.start..paging.end], columns, query))
        .maybe_child(pager(t, paging, EventHandler::new(move |p: usize| page.set(p))))
        .into_element()
}
