use freya::prelude::*;

use super::reveal::{scroll_to_reveal, visible_height};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Space kept between a revealed field and the keyboard or header edge.
const REVEAL_MARGIN: f32 = 16.;

/// A keyboard that covers more than this share of the window shrinks the header to its compact height.
const COMPACT_HEADER_SHARE: f32 = 0.4;

#[derive(Clone, PartialEq)]
struct RevealInput {
    target: Option<(f32, f32)>,
    viewport: f32,
}

/// A vertically scrolling area that keeps `target` (top and bottom in its content coordinates)
/// inside the part of itself that is visible. `viewport` is that visible height, which the caller
/// knows better than the scroll view does (a keyboard may cover the bottom of it).
#[derive(Clone, PartialEq)]
pub struct RevealScroll {
    target: Option<(f32, f32)>,
    viewport: f32,
    child: Element,
}

impl RevealScroll {
    pub fn new(child: impl IntoElement, target: Option<(f32, f32)>, viewport: f32) -> Self {
        Self { target, viewport, child: child.into_element() }
    }
}

impl Component for RevealScroll {
    fn render(&self) -> impl IntoElement {
        let mut scroll = use_scroll_controller(ScrollConfig::default);
        let input = use_reactive(&RevealInput { target: self.target, viewport: self.viewport });
        use_side_effect(move || {
            let RevealInput { target, viewport } = input.read().clone();
            if let Some((top, bottom)) = target {
                let (_, y): (i32, i32) = scroll.into();
                let next = scroll_to_reveal(-y as f32, top, bottom, viewport, REVEAL_MARGIN);
                scroll.scroll_to_y(-(next as i32));
            }
        });
        ScrollView::new_controlled(scroll).show_scrollbar(false).height(Size::fill()).child(self.child.clone())
    }
}

/// The mobile-settings pattern: a header with Back and a title, a scrolling body, a footer for the
/// actions. Used for every form and settings page on phones, handhelds and Deck mode.
///
/// Keyboard avoidance: `keyboard_inset` is the height of the on-screen keyboard (the host reads it
/// from the OS). While it is non-zero the footer and hints are hidden, the body grows a spacer so
/// the last field can scroll clear of the keyboard, and `reveal` (the focused field's top and
/// bottom in body coordinates) is scrolled into the part of the window that is still visible.
#[derive(Clone, PartialEq)]
pub struct FullScreenPage {
    title: String,
    window: (f32, f32),
    density: Density,
    keyboard_inset: f32,
    reveal: Option<(f32, f32)>,
    on_back: EventHandler<()>,
    body: Element,
    footer: Option<Element>,
    hints: Option<Element>,
    back_focused: bool,
    wide: bool,
    fixed: bool,
}

impl FullScreenPage {
    pub fn new(title: impl Into<String>, window: (f32, f32), density: Density, on_back: EventHandler<()>, body: impl IntoElement) -> Self {
        Self {
            title: title.into(),
            window,
            density,
            keyboard_inset: 0.,
            reveal: None,
            on_back,
            body: body.into_element(),
            footer: None,
            hints: None,
            back_focused: false,
            wide: false,
            fixed: false,
        }
    }

    pub fn keyboard_inset(mut self, inset: f32) -> Self {
        self.keyboard_inset = inset.max(0.);
        self
    }

    pub fn reveal(mut self, target: Option<(f32, f32)>) -> Self {
        self.reveal = target;
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_element());
        self
    }

    pub fn hints(mut self, hints: impl IntoElement) -> Self {
        self.hints = Some(hints.into_element());
        self
    }

    /// Use the full width instead of the centered reading column (two-pane settings).
    pub fn wide(mut self, wide: bool) -> Self {
        self.wide = wide;
        self
    }

    /// The body is laid out at the full viewport size and manages its own scrolling (two panes
    /// that scroll independently), so the page does not wrap it in a scroll view.
    pub fn fixed(mut self, fixed: bool) -> Self {
        self.fixed = fixed;
        self
    }

    pub fn back_focused(mut self, focused: bool) -> Self {
        self.back_focused = focused;
        self
    }

    /// Short windows put the footer's actions in the header and drop the footer row.
    fn actions_in_header(&self) -> bool {
        self.window.1 < SURFACE_SHORT_H
    }

    fn header_height(&self) -> f32 {
        header_height(self.window.1, self.keyboard_inset)
    }

    /// Height of the part of the body that is on screen and not under the keyboard, for a page
    /// with or without a footer and hints. Pages that scroll their own content (two-pane
    /// settings) need it to keep the focused row in view.
    pub fn body_viewport(window: (f32, f32), has_footer: bool, has_hints: bool, keyboard_inset: f32) -> f32 {
        let footer = if has_footer && window.1 >= SURFACE_SHORT_H { SURFACE_FOOTER_H } else { 0. };
        let hints = if has_hints { DECK_HINT_H + SURFACE_HINTS_PAD_B } else { 0. };
        visible_height(window.1, header_height(window.1, keyboard_inset), footer + hints, keyboard_inset)
    }
}

/// The header shrinks while a keyboard covers most of the window.
fn header_height(window_height: f32, keyboard_inset: f32) -> f32 {
    if keyboard_inset > window_height * COMPACT_HEADER_SHARE { SURFACE_HEADER_H_COMPACT } else { SURFACE_HEADER_H }
}

impl Component for FullScreenPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let controller = self.density == Density::Controller;
        let header_h = self.header_height();
        let keyboard_up = self.keyboard_inset > 0.;
        let in_header = self.actions_in_header();
        let viewport = Self::body_viewport(self.window, self.footer.is_some(), self.hints.is_some(), self.keyboard_inset);

        let on_back = self.on_back.clone();
        let title_style = if controller { TypeStyle::DeckHeading } else { TypeStyle::Heading };
        let background = if controller { t.deck_bg } else { t.bg_base };

        let header = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_3)
            .width(Size::fill())
            .height(Size::px(header_h))
            .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
            .background(if controller { t.deck_bg } else { t.bg_nav })
            .border(Border::new().fill(t.line).width(BorderWidth { bottom: 1., ..Default::default() }))
            .child(crate::deck::FocusFrame::new(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::Back)
                    .label("Back")
                    .size(if controller { ButtonSize::Controller } else { ButtonSize::for_density(self.density, false) })
                    .on_press(move |_| on_back.call(())),
                self.back_focused,
            ))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(title_style.text(self.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis)),
            )
            .maybe_child((in_header && !keyboard_up).then(|| self.footer.clone()).flatten());

        // The body column is centered and capped so lines stay readable on wide screens.
        let body = rect().vertical().width(Size::fill()).cross_align(Alignment::Center).child(
            rect()
                .vertical()
                .width(Size::fill())
                .maybe(!self.wide, |el| el.max_width(Size::px(SURFACE_MAX_W)))
                .padding(Gaps::new(SPACE_4, SPACE_4, SPACE_4, SPACE_4))
                .child(self.body.clone())
                .child(rect().height(Size::px(self.keyboard_inset))),
        );

        rect()
            .vertical()
            .content(Content::Flex)
            .width(Size::fill())
            .height(Size::fill())
            .background(background)
            .child(header)
            .child(rect().width(Size::fill()).height(Size::flex(1.)).child(if self.fixed {
                self.body.clone()
            } else {
                RevealScroll::new(body, self.reveal, viewport).into_element()
            }))
            .maybe_child((!keyboard_up && !in_header).then(|| self.footer.clone()).flatten().map(|footer| {
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .main_align(Alignment::End)
                    .spacing(SPACE_3)
                    .width(Size::fill())
                    .height(Size::px(SURFACE_FOOTER_H))
                    .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
                    .background(if controller { t.deck_bg } else { t.bg_nav })
                    .border(Border::new().fill(t.line).width(BorderWidth { top: 1., ..Default::default() }))
                    .child(footer)
            }))
            .maybe_child((!keyboard_up).then(|| self.hints.clone()).flatten().map(|hints| {
                rect()
                    .width(Size::fill())
                    .padding(Gaps::new(0., DECK_SAFE_X.min(self.window.0 / 8.), SURFACE_HINTS_PAD_B, DECK_SAFE_X.min(self.window.0 / 8.)))
                    .child(hints)
            }))
    }
}
