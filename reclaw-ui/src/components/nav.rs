use freya::prelude::*;

use super::{hoverable, pointer_cursor};
use crate::{metrics::*, nav::Section, prelude::*, typography::TypeStyle};

/// Where a navigation button goes: one of the tabs, or Settings after them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NavTarget {
    Tab(Section),
    Settings,
}

impl NavTarget {
    pub const ALL: [NavTarget; 5] =
        [Self::Tab(Section::Library), Self::Tab(Section::Catalog), Self::Tab(Section::Downloads), Self::Tab(Section::Mods), Self::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tab(section) => section.label(),
            Self::Settings => "Settings",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Tab(Section::Library) => IconName::Library,
            Self::Tab(Section::Catalog) => IconName::Catalog,
            Self::Tab(Section::Downloads) => IconName::Queue,
            Self::Tab(Section::Mods) => IconName::Mods,
            Self::Settings => IconName::Settings,
        }
    }
}

/// One component, three forms chosen from the container width:
/// `Top` at 1100px and up, `Rail` at 720-1099px, `Bottom` below 720px.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavMode {
    Top,
    Rail,
    Bottom,
}

impl From<LayoutClass> for NavMode {
    fn from(class: LayoutClass) -> Self {
        match class {
            LayoutClass::Wide => Self::Top,
            LayoutClass::Compact => Self::Rail,
            LayoutClass::Phone => Self::Bottom,
        }
    }
}

#[derive(Clone, PartialEq)]
struct NavButton {
    item: NavTarget,
    mode: NavMode,
    active: bool,
    count: Option<u32>,
    on_select: Option<EventHandler<NavTarget>>,
}

impl Component for NavButton {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let item = self.item;
        let (fg, size) = match self.mode {
            NavMode::Bottom => (if self.active { t.accent } else { t.ink_muted }, 22.),
            NavMode::Rail => (if self.active { t.ink } else { t.ink_muted }, 22.),
            NavMode::Top => (if self.active { t.ink } else { t.ink_muted }, 15.),
        };
        let fg = if hovering() && !self.active { t.ink } else { fg };

        let badge = self.count.filter(|n| *n > 0).map(|n| {
            rect()
                .padding(Gaps::new(0., 5., 0., 5.))
                .height(Size::px(16.))
                .center()
                .corner_radius(RADIUS_SM)
                .background(t.accent)
                .child(TypeStyle::Mono.text(n.to_string(), t.on_accent))
        });

        let body = match self.mode {
            NavMode::Top => rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(SPACE_2)
                .height(Size::fill())
                .padding(Gaps::new(0., SPACE_1, 0., SPACE_1))
                .border(
                    Border::new()
                        .fill(if self.active { t.accent } else { super::CLEAR })
                        .width(BorderWidth { bottom: 2., ..Default::default() })
                        .alignment(BorderAlignment::Inner),
                )
                .child(icon(item.icon(), size, fg))
                .child(TypeStyle::Eyebrow.text(item.label(), fg))
                .maybe_child(badge),
            NavMode::Rail => rect()
                .width(Size::px(TARGET_MIN))
                .height(Size::px(TARGET_MIN))
                .center()
                .corner_radius(RADIUS_MD)
                .background(if self.active { t.bg_raised } else { super::CLEAR })
                .maybe(self.active, |el| {
                    el.border(
                        Border::new()
                            .fill(t.accent)
                            .width(BorderWidth { left: 3., ..Default::default() })
                            .alignment(BorderAlignment::Inner),
                    )
                })
                .child(icon(item.icon(), size, fg)),
            NavMode::Bottom => rect()
                .vertical()
                .center()
                .spacing(2.)
                .min_width(Size::px(TARGET_MIN))
                .min_height(Size::px(TARGET_MIN))
                .child(icon(item.icon(), size, fg))
                .child(TypeStyle::Eyebrow.text(item.label(), fg))
                .maybe_child(badge),
        };

        let body = body
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(item.label())
            .map(self.on_select.clone(), |el, handler| el.on_press(move |_| handler.call(item)));
        let body = pointer_cursor(hoverable(body, hovering));

        if self.mode == NavMode::Rail {
            TooltipContainer::new(Tooltip::new_text(item.label())).position(AttachedPosition::Right).child(body).into_element()
        } else {
            body.into_element()
        }
    }

    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.item)
    }
}

#[derive(Clone, PartialEq)]
pub struct Nav {
    mode: NavMode,
    active: NavTarget,
    downloads: Option<u32>,
    trailing: Option<Element>,
    actions: Option<Element>,
    on_select: Option<EventHandler<NavTarget>>,
}

impl Nav {
    pub fn new(mode: NavMode, active: NavTarget) -> Self {
        Self { mode, active, downloads: None, trailing: None, actions: None, on_select: None }
    }

    /// Shows a count badge on Downloads while the queue is active.
    pub fn downloads(mut self, count: u32) -> Self {
        self.downloads = Some(count);
        self
    }

    /// Top mode only: a slot pushed to the right edge, used for the search field.
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_element());
        self
    }

    /// Top mode only: buttons placed before the search field, such as the switch to Deck mode.
    pub fn actions(mut self, element: impl IntoElement) -> Self {
        self.actions = Some(element.into_element());
        self
    }

    pub fn on_select(mut self, handler: impl Into<EventHandler<NavTarget>>) -> Self {
        self.on_select = Some(handler.into());
        self
    }
}

impl Component for Nav {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let buttons = NavTarget::ALL.map(|item| NavButton {
            item,
            mode: self.mode,
            active: item == self.active,
            count: (item == NavTarget::Tab(Section::Downloads)).then_some(self.downloads).flatten(),
            on_select: self.on_select.clone(),
        });

        match self.mode {
            NavMode::Top => rect()
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .spacing(SPACE_5)
                .width(Size::fill())
                .height(Size::px(TOPBAR_H))
                .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
                .background(t.bg_nav)
                .border(Border::new().fill(t.line).width(BorderWidth { bottom: 1., ..Default::default() }))
                .child(TypeStyle::Heading.text("Reclaw", t.ink))
                // The tabs sit closer to each other than to the title and the actions: at the narrowest wide window (1100 px) the row
                // has to fit the title, five tabs with a badge, two actions and the search box.
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .height(Size::fill())
                        .spacing(SPACE_3)
                        .children(buttons.into_iter().map(IntoElement::into_element)),
                )
                .child(rect().width(Size::flex(1.)))
                .maybe_child(self.actions.clone().map(|el| rect().padding(Gaps::new(0., SPACE_2, 0., 0.)).child(el)))
                .maybe_child(self.trailing.clone().map(|el| rect().width(Size::px(240.)).child(el)))
                .into_element(),
            NavMode::Rail => rect()
                .vertical()
                .cross_align(Alignment::Center)
                .spacing(SPACE_2)
                .width(Size::px(RAIL_W))
                .height(Size::fill())
                .padding(Gaps::new(SPACE_3, 0., SPACE_3, 0.))
                .background(t.bg_nav)
                .border(Border::new().fill(t.line).width(BorderWidth { right: 1., ..Default::default() }))
                .children(buttons.into_iter().map(IntoElement::into_element))
                .into_element(),
            NavMode::Bottom => rect()
                .horizontal()
                .main_align(Alignment::SpaceAround)
                .cross_align(Alignment::Center)
                .width(Size::fill())
                .height(Size::px(TABBAR_H))
                .background(t.bg_nav)
                .border(Border::new().fill(t.line).width(BorderWidth { top: 1., ..Default::default() }))
                .children(buttons.into_iter().map(IntoElement::into_element))
                .into_element(),
        }
    }
}
