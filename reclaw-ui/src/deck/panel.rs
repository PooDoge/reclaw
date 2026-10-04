use freya::{animation::*, prelude::*};
use reclaw_input::{ControllerInfo, FocusId, PowerState};

use super::{
    Section,
    banner::elapsed_label,
    focus::FocusFrame,
    state::{MENU, MenuEntry, ids},
};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelSide {
    Left,
    Right,
}

/// Overlay that slides in from an edge over the page. It never shifts the layout beneath, draws
/// the `scrim` token over the rest of the window and closes on a click outside.
#[derive(Clone, PartialEq)]
pub struct SlidePanel {
    side: PanelSide,
    open: bool,
    window: (f32, f32),
    on_close: EventHandler<()>,
    child: Element,
}

impl SlidePanel {
    pub fn new(
        side: PanelSide,
        open: bool,
        window: (f32, f32),
        on_close: EventHandler<()>,
        child: impl IntoElement,
    ) -> Self {
        Self {
            side,
            open,
            window,
            on_close,
            child: child.into_element(),
        }
    }
}

impl Component for SlidePanel {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let anim = use_animation_transition(self.open, |from: bool, to: bool| {
            AnimNum::new(if from { 1. } else { 0. }, if to { 1. } else { 0. })
                .time(280)
                .ease(Ease::Out)
                .function(Function::Expo)
        });
        let k = anim.read().value();
        if !self.open && k <= 0.001 {
            return rect().into_element();
        }
        let (w, h) = self.window;
        let x = match self.side {
            PanelSide::Left => -DECK_PANEL_W * (1. - k),
            PanelSide::Right => w - DECK_PANEL_W * k,
        };
        let scrim = t.scrim;
        let faded = Color::from_argb(
            (scrim.a() as f32 * k) as u8,
            scrim.r(),
            scrim.g(),
            scrim.b(),
        );
        let on_close = self.on_close.clone();

        // The root is itself absolute at the window origin: absolute children are placed relative to
        // their parent's origin, and a plain root would sit after the backdrop in the flow.
        // Layer::Overlay paints this above the whole page, including its own scaled focus frames.
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
                    .background(faded)
                    .on_press(move |_| on_close.call(())),
            )
            .child(
                rect()
                    .position(Position::new_absolute().top(0.).left(x))
                    .width(Size::px(DECK_PANEL_W))
                    .height(Size::px(h))
                    .background(t.bg_panel)
                    .border(
                        Border::new()
                            .fill(t.line)
                            .width(1.)
                            .alignment(BorderAlignment::Inner),
                    )
                    .shadow(
                        Shadow::new()
                            .blur(32.)
                            .y(12.)
                            .color(Color::from_argb(128, 0, 0, 0)),
                    )
                    .child(self.child.clone()),
            )
            .into_element()
    }
}

/// One list row in a panel: icon, label, accent bar when focused (no scale: rows stay aligned).
#[derive(Clone, PartialEq)]
struct DeckRow {
    icon: IconName,
    label: &'static str,
    current: bool,
    focused: bool,
    on_press: EventHandler<()>,
}

impl Component for DeckRow {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let on_press = self.on_press.clone();
        let fg = if self.focused || self.current {
            t.ink
        } else {
            t.ink_muted
        };
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(SPACE_4)
            .width(Size::fill())
            .height(Size::px(DECK_ROW_H))
            .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
            .background(if self.focused {
                t.bg_raised
            } else {
                crate::components::CLEAR
            })
            .border(
                Border::new()
                    .fill(if self.focused {
                        t.accent
                    } else {
                        crate::components::CLEAR
                    })
                    .width(BorderWidth {
                        left: 4.,
                        ..Default::default()
                    })
                    .alignment(BorderAlignment::Inner),
            )
            .child(icon(self.icon, 24., fg))
            .child(TypeStyle::DeckBody.text(self.label, fg))
            .on_press(move |_| on_press.call(()))
    }
}

fn menu_label(entry: MenuEntry) -> (&'static str, IconName) {
    match entry {
        MenuEntry::Section(Section::Library) => ("Library", IconName::Library),
        MenuEntry::Section(Section::Catalog) => ("Catalog", IconName::Catalog),
        MenuEntry::Section(Section::Downloads) => ("Downloads", IconName::Queue),
        MenuEntry::Section(Section::Mods) => ("Mods", IconName::Mods),
        MenuEntry::Settings => ("Settings", IconName::Settings),
        MenuEntry::SwitchToDesktop => ("Switch to desktop mode", IconName::Desktop),
    }
}

/// Left panel: sections, settings, and the way out to desktop mode.
#[derive(Clone, PartialEq)]
pub struct MainMenu {
    section: Section,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl MainMenu {
    pub fn new(
        section: Section,
        focus: FocusId,
        ring_visible: bool,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self {
            section,
            focus,
            ring_visible,
            on_click,
        }
    }
}

impl Component for MainMenu {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let rows = MENU.iter().enumerate().map(|(i, entry)| {
            let (label, icon) = menu_label(*entry);
            let id = ids::menu(i);
            let on_click = self.on_click.clone();
            DeckRow {
                icon,
                label,
                current: *entry == MenuEntry::Section(self.section),
                focused: self.ring_visible && self.focus == id,
                on_press: EventHandler::new(move |_| on_click.call(id)),
            }
            .into_element()
        });
        rect()
            .vertical()
            .width(Size::fill())
            .child(
                rect()
                    .height(Size::px(DECK_TABS_H))
                    .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                    .main_align(Alignment::Center)
                    .child(TypeStyle::DeckHeading.text("Reclaw", t.ink)),
            )
            .children(rows)
    }
}

fn power_label(power: PowerState) -> Option<String> {
    match power {
        PowerState::Unknown => None,
        PowerState::Wired => Some("Wired".into()),
        PowerState::Full => Some("Battery full".into()),
        PowerState::Charging(p) => Some(format!("Charging, {p}%")),
        PowerState::Discharging(p) => Some(format!("Battery {p}%")),
    }
}

/// Right panel: the running app, the connected controller, the download queue.
#[derive(Clone, PartialEq)]
pub struct QuickAccess {
    active: Option<GameEntry>,
    controller: Option<ControllerInfo>,
    queue: usize,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl QuickAccess {
    pub fn new(
        active: Option<GameEntry>,
        controller: Option<ControllerInfo>,
        queue: usize,
        focus: FocusId,
        ring_visible: bool,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self {
            active,
            controller,
            queue,
            focus,
            ring_visible,
            on_click,
        }
    }
}

impl Component for QuickAccess {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let section = |text: &'static str| {
            rect()
                .padding(Gaps::new(SPACE_5, SPACE_5, SPACE_2, SPACE_5))
                .child(TypeStyle::Eyebrow.text(text, t.ink_muted))
        };

        let now_playing = self.active.clone().map(|game| {
            let stopping = matches!(game.run, reclaw_runtime::RunState::Stopping { .. });
            let (a, b) = (self.on_click.clone(), self.on_click.clone());
            rect()
                .vertical()
                .spacing(SPACE_3)
                .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                .child(
                    TypeStyle::DeckLabel
                        .text(game.title.clone(), t.ink)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                )
                .child(TypeStyle::DeckMeta.text(elapsed_label(&game.run), t.ink_muted))
                .child(
                    rect()
                        .horizontal()
                        .spacing(SPACE_4)
                        .padding(Gaps::new(SPACE_2, 0., SPACE_2, 0.))
                        .child(FocusFrame::new(
                            ActionButton::install()
                                .icon(IconName::Play)
                                .label("Resume")
                                .size(ButtonSize::Controller)
                                .enabled(!stopping)
                                .on_press(move |_| a.call(ids::QA_RESUME)),
                            self.ring_visible && self.focus == ids::QA_RESUME,
                        ))
                        .child(FocusFrame::new(
                            ActionButton::new(ButtonVariant::Danger)
                                .icon(IconName::Stop)
                                .label(if stopping { "Force quit" } else { "Stop" })
                                .size(ButtonSize::Controller)
                                .on_press(move |_| b.call(ids::QA_STOP)),
                            self.ring_visible && self.focus == ids::QA_STOP,
                        )),
                )
        });

        let controller = match &self.controller {
            Some(c) => rect()
                .vertical()
                .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                .child(
                    TypeStyle::DeckBody
                        .text(c.name.clone(), t.ink)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                )
                .maybe_child(
                    power_label(c.power).map(|p| TypeStyle::DeckMeta.text(p, t.ink_muted)),
                ),
            None => rect()
                .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                .child(TypeStyle::DeckBody.text("No controller connected", t.ink_muted)),
        };

        let on_click = self.on_click.clone();
        rect()
            .vertical()
            .width(Size::fill())
            .child(
                rect()
                    .height(Size::px(DECK_TABS_H))
                    .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                    .main_align(Alignment::Center)
                    .child(TypeStyle::DeckHeading.text("Quick access", t.ink)),
            )
            .maybe(self.active.is_some(), |el| el.child(section("Now playing")))
            .maybe_child(now_playing)
            .child(section("Controller"))
            .child(controller)
            .child(section("Downloads"))
            .child(DeckRow {
                icon: IconName::Queue,
                label: if self.queue == 0 {
                    "Nothing in the queue"
                } else {
                    "Open the download queue"
                },
                current: false,
                focused: self.ring_visible && self.focus == ids::QA_DOWNLOADS,
                on_press: EventHandler::new(move |_| on_click.call(ids::QA_DOWNLOADS)),
            })
    }
}
