use freya::prelude::*;
use reclaw_input::FocusId;

use crate::{
    deck::{FocusFrame, LaunchButton, ids},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

/// Game page: project, title, badges, the launch verb(s), and the secondary actions.
#[derive(Clone, PartialEq)]
pub struct GamePage {
    game: GameEntry,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl GamePage {
    pub fn new(game: GameEntry, focus: FocusId, ring_visible: bool, on_click: EventHandler<FocusId>) -> Self {
        Self { game, focus, ring_visible, on_click }
    }
}

impl Component for GamePage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let g = &self.game;
        let (a, b) = (self.on_click.clone(), self.on_click.clone());
        let badge = if g.run.is_active() { StatusBadge::running() } else { StatusBadge::new(g.status) };

        rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::px(720.))
            .padding(Gaps::new(SPACE_6, 0., 0., 0.))
            .child(TypeStyle::Eyebrow.text(g.project.clone(), t.accent))
            .child(TypeStyle::DeckTitle.text(g.title.clone(), t.ink))
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_4)
                    .child(badge)
                    .maybe(!g.version.is_empty(), |el| el.child(TypeStyle::Mono.text(g.version.clone(), t.ink_muted)))
                    .child(TypeStyle::DeckMeta.text(g.source.host(), t.ink_muted)),
            )
            .child(rect().height(Size::px(SPACE_5)))
            .child(LaunchButton::new(g.clone(), self.focus, self.ring_visible, ids::GAME_PRIMARY, ids::GAME_STOP, self.on_click.clone()))
            .child(
                rect()
                    .horizontal()
                    .spacing(SPACE_4)
                    .child(FocusFrame::new(
                        ActionButton::new(ButtonVariant::Secondary)
                            .icon(IconName::Folder)
                            .label("Open folder")
                            .size(ButtonSize::Controller)
                            .on_press(move |_| a.call(ids::GAME_FOLDER)),
                        self.ring_visible && self.focus == ids::GAME_FOLDER,
                    ))
                    .child(FocusFrame::new(
                        ActionButton::new(ButtonVariant::Secondary)
                            .icon(IconName::Settings)
                            .label("Manage")
                            .size(ButtonSize::Controller)
                            .on_press(move |_| b.call(ids::GAME_MANAGE)),
                        self.ring_visible && self.focus == ids::GAME_MANAGE,
                    )),
            )
    }
}
