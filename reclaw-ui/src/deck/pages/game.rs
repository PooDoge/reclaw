use freya::prelude::*;
use reclaw_input::FocusId;

use crate::{
    community::{Rating, Tone},
    deck::{FocusFrame, LaunchButton, ids},
    metrics::*,
    prelude::*,
    store::use_community_of,
    typography::TypeStyle,
};

/// Game page: project, title, badges, how it runs (from quiverlauncher.com), the launch verb(s), and the secondary actions.
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
        let community = use_community_of(g.id);
        // What players said and the verified release, in one line under the badges; nothing for a game the site does not list.
        let site = community.app.map(|app| {
            let rating = Rating::of(&app);
            let color = match rating.tone {
                Tone::Positive => t.ok,
                Tone::Caution => t.warn,
                Tone::Negative => t.danger,
                Tone::Muted => t.ink_muted,
            };
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(SPACE_4)
                .child(TypeStyle::DeckMeta.text(rating.line(), color))
                .maybe_child(
                    app.verified
                        .filter(|v| !v.version.trim().is_empty())
                        .map(|v| TypeStyle::DeckMeta.text(format!("{} verified", v.version.trim()), t.ink_muted)),
                )
        });
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
            .maybe_child(site)
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
