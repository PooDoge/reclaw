//! Small pieces the desktop pages share: the header of a page below a tab, the scrolling body,
//! cards and headings, the capsule grid.
//!
//! * `not_found`: the page for an id nothing knows; `mod_row`: one mod in a list
mod mod_row;
mod not_found;

pub use mod_row::{ModRow, compact_count, mod_action, mod_effect};
pub use not_found::NotFound;

use freya::prelude::*;

use crate::{
    desktop::DesktopEnv,
    metrics::*,
    nav::{Route, use_nav},
    prelude::*,
    typography::TypeStyle,
};

/// The header of a page that is not a top-level tab: Back, an eyebrow naming where it is, the title,
/// and a slot for actions on the right.
#[derive(Clone, PartialEq)]
pub struct PageHeader {
    pub title: String,
    pub eyebrow: Option<String>,
    pub trailing: Option<Element>,
}

impl PageHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), eyebrow: None, trailing: None }
    }

    pub fn eyebrow(mut self, eyebrow: impl Into<String>) -> Self {
        self.eyebrow = Some(eyebrow.into());
        self
    }

    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_element());
        self
    }
}

impl Component for PageHeader {
    fn render(&self) -> impl IntoElement {
        let (t, nav) = (use_reclaw(), use_nav());
        rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(ActionButton::new(ButtonVariant::Ghost).icon(IconName::Back).label("Back").enabled(nav.can_go_back()).on_press(
                move |_| {
                    nav.back();
                },
            ))
            .child(
                rect()
                    .vertical()
                    .width(Size::flex(1.))
                    .maybe_child(self.eyebrow.clone().map(|e| TypeStyle::Eyebrow.text(e, t.accent)))
                    .child(TypeStyle::TitlePage.text(self.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis)),
            )
            .maybe_child(self.trailing.clone())
    }
}

/// Just the Back button, for a page that carries its own title (the Game page's banner). Back is
/// also the mouse's back button, Alt+Left, Escape and the pad's B; this is the one a touch screen has.
#[derive(PartialEq)]
pub struct BackBar {}

impl Component for BackBar {
    fn render(&self) -> impl IntoElement {
        let nav = use_nav();
        rect().horizontal().width(Size::fill()).child(
            ActionButton::new(ButtonVariant::Ghost).icon(IconName::Back).label("Back").enabled(nav.can_go_back()).on_press(move |_| {
                nav.back();
            }),
        )
    }
}

/// A scrolling page body with the gutter the layout class calls for.
pub fn page_scroll(env: &DesktopEnv, body: impl IntoElement) -> Element {
    let gutter = if env.class == LayoutClass::Wide { SPACE_5 } else { SPACE_4 };
    ScrollView::new()
        .show_scrollbar(false)
        .child(rect().vertical().spacing(gutter).width(Size::fill()).padding(gutter).child(body))
        .into_element()
}

/// A section heading inside a page.
pub fn heading(t: &Reclaw, text: &'static str) -> Rect {
    rect().width(Size::fill()).child(TypeStyle::Eyebrow.text(text, t.ink_subtle))
}

/// Capsules in fixed-column rows of fluid capsules, so the last row keeps its column width.
/// Pressing one opens that game's page.
pub fn capsule_grid(games: &[GameEntry], columns: usize, selected: Option<State<Option<u32>>>) -> Rect {
    let nav = use_nav();
    rect().vertical().spacing(SPACE_3).width(Size::fill()).children(games.chunks(columns.max(1)).enumerate().map(|(row, chunk)| {
        let mut line = rect().horizontal().content(Content::Flex).spacing(SPACE_3).width(Size::fill());
        for game in chunk {
            let id = game.id;
            let current = selected.and_then(|s| *s.peek());
            line = line.child(
                rect().width(Size::flex(1.)).child(
                    GameCapsule::new(game.clone())
                        .fluid(true)
                        .selected(current == Some(id))
                        .on_press(move |_| {
                            if let Some(mut selected) = selected {
                                selected.set(Some(id));
                            }
                            nav.open(Route::Game { id });
                        })
                        .key(id),
                ),
            );
        }
        for _ in chunk.len()..columns {
            line = line.child(rect().width(Size::flex(1.)));
        }
        line.key(row).into_element()
    }))
}

/// How many capsules fit across the window.
pub fn columns_for(env: &DesktopEnv) -> usize {
    let gutter = if env.class == LayoutClass::Wide { SPACE_5 } else { SPACE_4 };
    let usable = env.window.0 - 2. * gutter - if env.class == LayoutClass::Compact { RAIL_W } else { 0. };
    ((usable / (CAPSULE_W + SPACE_3)).floor() as usize).clamp(2, 8)
}

/// Lines in a bordered panel.
pub fn card(t: &Reclaw, lines: impl IntoIterator<Item = Element>) -> Rect {
    rect()
        .vertical()
        .spacing(SPACE_2)
        .width(Size::fill())
        .padding(SPACE_4)
        .background(t.bg_panel)
        .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
        .corner_radius(RADIUS_MD)
        .children(lines)
}

/// The title row of a top-level page: its name and a short summary, and on layouts without the top
/// bar's search field (compact and phone), a search box below.
pub fn tab_header(t: &Reclaw, env: &DesktopEnv, title: &'static str, summary: String, search: State<String>) -> Rect {
    rect()
        .vertical()
        .spacing(SPACE_3)
        .width(Size::fill())
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::End)
                .spacing(SPACE_3)
                .child(TypeStyle::TitlePage.text(title, t.ink))
                .child(TypeStyle::Meta.text(summary, t.ink_subtle)),
        )
        .maybe(env.class != LayoutClass::Wide, |el| el.child(SearchField::new(search).density(env.density)))
}

/// Nothing to show, and why: an empty list is explained, never blank.
pub fn empty_state(t: &Reclaw, title: &'static str, text: &'static str) -> Rect {
    rect()
        .vertical()
        .spacing(SPACE_2)
        .width(Size::fill())
        .padding(SPACE_6)
        .center()
        .child(TypeStyle::Heading.text(title, t.ink))
        .child(TypeStyle::Body.text(text, t.ink_muted))
}
