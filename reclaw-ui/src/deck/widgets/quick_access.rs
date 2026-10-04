use freya::prelude::*;
use reclaw_input::{ControllerInfo, FocusId, PowerState};

use super::deck_row::DeckRow;
use crate::{
    deck::{FocusFrame, QA_RECENTS, elapsed_label, ids},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

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
    /// Titles of the pages visited lately, newest first; the first few are listed.
    recents: Vec<String>,
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
        Self { active, controller, queue, recents: Vec::new(), focus, ring_visible, on_click }
    }

    /// The pages to jump back to, by title.
    pub fn recents(mut self, recents: Vec<String>) -> Self {
        self.recents = recents;
        self
    }
}

impl Component for QuickAccess {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let section = |text: &'static str| {
            rect().padding(Gaps::new(SPACE_5, SPACE_5, SPACE_2, SPACE_5)).child(TypeStyle::Eyebrow.text(text, t.ink_muted))
        };

        let now_playing = self.active.clone().map(|game| {
            let stopping = matches!(game.run, reclaw_runtime::RunState::Stopping { .. });
            let (a, b) = (self.on_click.clone(), self.on_click.clone());
            rect()
                .vertical()
                .spacing(SPACE_3)
                .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                .child(TypeStyle::DeckLabel.text(game.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis))
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
                .child(TypeStyle::DeckBody.text(c.name.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis))
                .maybe_child(power_label(c.power).map(|p| TypeStyle::DeckMeta.text(p, t.ink_muted))),
            None => {
                rect().padding(Gaps::new(0., SPACE_5, 0., SPACE_5)).child(TypeStyle::DeckBody.text("No controller connected", t.ink_muted))
            }
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
                label: if self.queue == 0 { "Nothing in the queue" } else { "Open the download queue" }.into(),
                current: false,
                focused: self.ring_visible && self.focus == ids::QA_DOWNLOADS,
                on_press: EventHandler::new(move |_| on_click.call(ids::QA_DOWNLOADS)),
            })
            .maybe(!self.recents.is_empty(), |el| el.child(section("Recent")))
            .children(self.recents.iter().take(QA_RECENTS).enumerate().map(|(i, title)| {
                let (id, on_click) = (ids::qa_recent(i), self.on_click.clone());
                DeckRow {
                    icon: IconName::Clock,
                    label: title.clone().into(),
                    current: false,
                    focused: self.ring_visible && self.focus == id,
                    on_press: EventHandler::new(move |_| on_click.call(id)),
                }
                .into_element()
            }))
    }
}
