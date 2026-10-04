use freya::prelude::*;

use super::menu::{EntryKind, MenuState};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

#[derive(Clone, PartialEq, Debug)]
pub struct MenuRowView {
    pub label: String,
    pub chevron: bool,
    pub separator_before: bool,
    pub enabled: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MenuLevelView {
    pub title: String,
    pub rows: Vec<MenuRowView>,
    pub focus: usize,
    pub open: Option<usize>,
}

impl MenuLevelView {
    /// The visible levels of a menu state, outermost first.
    pub fn from_state<A: Clone>(state: &MenuState<A>) -> Vec<Self> {
        state
            .levels()
            .into_iter()
            .map(|level| Self {
                title: level.title.to_string(),
                focus: level.focus,
                open: level.open,
                rows: level
                    .entries
                    .iter()
                    .map(|e| MenuRowView {
                        label: e.label.clone(),
                        chevron: matches!(e.kind, EntryKind::Submenu { .. }),
                        separator_before: e.separator_before,
                        enabled: e.enabled,
                    })
                    .collect(),
            })
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MenuPlacement {
    /// Over a darkened screen, with the title above: touch, gamepad, phones.
    Centered,
    /// A popover at a point, no dimming: pointer on a desktop-sized window.
    Anchored { x: f32, y: f32 },
}

/// Cascading option menu. Centered placement mirrors Big Picture: the screen darkens, the title
/// sits above, and a submenu opens to the right of its parent, which stays visible with its open
/// row marked. On a narrow window only the deepest level shows, so a drill-down replaces it.
#[derive(Clone, PartialEq)]
pub struct ModalMenu {
    levels: Vec<MenuLevelView>,
    placement: MenuPlacement,
    window: (f32, f32),
    density: Density,
    ring_visible: bool,
    on_pick: EventHandler<(usize, usize)>,
    on_dismiss: EventHandler<()>,
}

impl ModalMenu {
    pub fn new(
        levels: Vec<MenuLevelView>,
        placement: MenuPlacement,
        window: (f32, f32),
        density: Density,
        on_pick: EventHandler<(usize, usize)>,
        on_dismiss: EventHandler<()>,
    ) -> Self {
        Self { levels, placement, window, density, ring_visible: true, on_pick, on_dismiss }
    }

    /// Whether the focused row is drawn highlighted (hidden after pointer input).
    pub fn ring_visible(mut self, visible: bool) -> Self {
        self.ring_visible = visible;
        self
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Metrics {
    row_h: f32,
    width: f32,
    text: TypeStyle,
}

fn metrics(density: Density, anchored: bool, window_w: f32) -> Metrics {
    if anchored {
        return Metrics { row_h: 40., width: 280., text: TypeStyle::Body };
    }
    let row_h = if density == Density::Controller { DECK_ROW_H } else { 56. };
    Metrics { row_h, width: DECK_MENU_W.min(window_w - 48.), text: TypeStyle::DeckBody }
}

#[derive(Clone, PartialEq)]
struct Row {
    view: MenuRowView,
    level: usize,
    index: usize,
    focused: bool,
    /// The row whose submenu is open in the next column.
    open: bool,
    m: Metrics,
    on_pick: EventHandler<(usize, usize)>,
}

impl Component for Row {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let v = &self.view;
        let (fill, fg) = if !v.enabled {
            (crate::components::CLEAR, t.ink_subtle)
        } else if self.focused {
            // Big Picture inverts the focused row: unmistakable at ten feet.
            (t.ink, t.deck_bg)
        } else if self.open {
            (t.ink_muted, t.deck_bg)
        } else if hovering() {
            (t.bg_raised, t.ink)
        } else {
            (crate::components::CLEAR, t.ink)
        };
        let (level, index, on_pick) = (self.level, self.index, self.on_pick.clone());
        let enabled = v.enabled;
        let row = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .height(Size::px(self.m.row_h))
            .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
            .background(fill)
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(self.m.text.text(v.label.clone(), fg).max_lines(1).text_overflow(TextOverflow::Ellipsis)),
            )
            .maybe(v.chevron, |el| el.child(icon(IconName::Chevron, 20., fg)))
            .maybe(enabled, |el| el.on_press(move |_| on_pick.call((level, index))));
        super::super::components::hoverable(row, hovering)
    }
}

impl Component for ModalMenu {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (w, h) = self.window;
        let anchored = matches!(self.placement, MenuPlacement::Anchored { .. });
        let m = metrics(self.density, anchored, w);
        let on_dismiss = self.on_dismiss.clone();

        // Narrow windows show only the deepest level.
        let first_visible = if self.levels.len() > 1 && w < 2. * m.width + 96. { self.levels.len() - 1 } else { 0 };
        let deepest_title = self.levels.last().map(|l| l.title.clone()).unwrap_or_default();
        let last = self.levels.len().saturating_sub(1);

        // Height of a level's rows, so a column is as tall as its content rather than as the window.
        let content_h = |level: &MenuLevelView| {
            level.rows.iter().enumerate().map(|(i, v)| m.row_h + if v.separator_before && i > 0 { 4. } else { 0. }).sum::<f32>()
        };
        let max_h = (h - 200.).max(m.row_h * 3.);
        let tallest = self.levels.iter().skip(first_visible).map(|l| content_h(l).min(max_h)).fold(0., f32::max);

        let columns = self.levels.iter().enumerate().skip(first_visible).map(|(li, level)| {
            let is_last = li == last;
            let rows = level.rows.iter().enumerate().map(|(i, view)| {
                let row = Row {
                    view: view.clone(),
                    level: li,
                    index: i,
                    focused: is_last && self.ring_visible && level.focus == i,
                    open: level.open == Some(i) && !is_last,
                    m,
                    on_pick: self.on_pick.clone(),
                };
                rect()
                    .vertical()
                    .width(Size::fill())
                    .maybe(view.separator_before && i > 0, |el| {
                        el.child(rect().width(Size::fill()).height(Size::px(4.)).background(t.deck_bg))
                    })
                    .child(row)
                    .into_element()
            });
            let column_h = content_h(level).min(max_h);
            rect()
                .width(Size::px(m.width))
                .background(t.bg_panel)
                .maybe(anchored, |el| {
                    el.border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
                        .corner_radius(RADIUS_MD)
                        .shadow(Shadow::new().blur(32.).y(12.).color(Color::from_argb(128, 0, 0, 0)))
                })
                .child(
                    ScrollView::new()
                        .show_scrollbar(false)
                        .height(Size::px(column_h))
                        .child(rect().vertical().width(Size::fill()).children(rows)),
                )
                .into_element()
        });

        match self.placement {
            MenuPlacement::Centered => rect()
                .position(Position::new_absolute().top(0.).left(0.))
                .layer(Layer::Overlay)
                .width(Size::px(w))
                .height(Size::px(h))
                .child(
                    rect()
                        .position(Position::new_absolute().top(0.).left(0.))
                        .width(Size::px(w))
                        .height(Size::px(h))
                        .background(Color::from_argb(217, 5, 8, 12))
                        .on_press(move |_| on_dismiss.call(())),
                )
                .child(
                    rect()
                        .position(Position::new_absolute().top(0.).left(0.))
                        .width(Size::px(w))
                        .height(Size::px(h))
                        .interactive(Interactive::No)
                        .center()
                        .child(
                            rect()
                                .vertical()
                                .cross_align(Alignment::Center)
                                .spacing(SPACE_5)
                                .child(TypeStyle::DeckHeading.text(deepest_title, t.ink))
                                .child(rect().horizontal().cross_align(Alignment::Start).interactive(Interactive::Yes).children(columns)),
                        ),
                )
                .into_element(),
            MenuPlacement::Anchored { x, y } => {
                let x = x.min(w - m.width - 8.).max(8.);
                // Keep the whole menu on screen: slide up if it would run off the bottom.
                let y = y.min(h - 8. - tallest).max(8.);
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .layer(Layer::Overlay)
                    .width(Size::px(w))
                    .height(Size::px(h))
                    .child(
                        rect()
                            .position(Position::new_absolute().top(0.).left(0.))
                            .width(Size::px(w))
                            .height(Size::px(h))
                            .on_press(move |_| on_dismiss.call(())),
                    )
                    .child(rect().position(Position::new_absolute().top(y).left(x)).horizontal().children(columns))
                    .into_element()
            }
        }
    }
}
