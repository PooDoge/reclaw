use freya::prelude::*;
use reclaw_input::FocusId;

use super::{
    focus::FocusFrame,
    launch::{LaunchVerb, failure_message, launch_verb, shows_stop_pair},
};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

fn verb_icon(verb: LaunchVerb) -> IconName {
    match verb {
        LaunchVerb::Install | LaunchVerb::Update => IconName::Download,
        LaunchVerb::Installing | LaunchVerb::Starting => IconName::Queue,
        LaunchVerb::Play | LaunchVerb::Resume => IconName::Play,
        LaunchVerb::Stop | LaunchVerb::ForceQuit => IconName::Stop,
        LaunchVerb::Retry => IconName::Refresh,
    }
}

fn verb_variant(verb: LaunchVerb) -> ButtonVariant {
    match verb {
        LaunchVerb::Stop | LaunchVerb::ForceQuit => ButtonVariant::Danger,
        _ => ButtonVariant::Install,
    }
}

/// The primary verb for a game in controller density, with a Stop button beside Resume while the
/// app runs, and a plain-words reason under it when the last run ended badly.
#[derive(Clone, PartialEq)]
pub struct LaunchButton {
    game: GameEntry,
    focus: FocusId,
    ring_visible: bool,
    primary_id: FocusId,
    stop_id: FocusId,
    on_click: EventHandler<FocusId>,
}

impl LaunchButton {
    pub fn new(
        game: GameEntry,
        focus: FocusId,
        ring_visible: bool,
        primary_id: FocusId,
        stop_id: FocusId,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self {
            game,
            focus,
            ring_visible,
            primary_id,
            stop_id,
            on_click,
        }
    }
}

impl Component for LaunchButton {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let verb = launch_verb(self.game.status, &self.game.run, true);
        let (primary, stop) = (self.primary_id, self.stop_id);
        let (on_a, on_b) = (self.on_click.clone(), self.on_click.clone());

        rect()
            .vertical()
            .spacing(SPACE_3)
            .child(
                rect()
                    .horizontal()
                    .spacing(SPACE_4)
                    .child(FocusFrame::new(
                        ActionButton::new(verb_variant(verb))
                            .icon(verb_icon(verb))
                            .label(verb.label())
                            .size(ButtonSize::Controller)
                            .enabled(verb.enabled())
                            .on_press(move |_| on_a.call(primary)),
                        self.ring_visible && self.focus == primary,
                    ))
                    .maybe(shows_stop_pair(&self.game.run, true), |el| {
                        el.child(FocusFrame::new(
                            ActionButton::new(ButtonVariant::Danger)
                                .icon(IconName::Stop)
                                .label("Stop")
                                .size(ButtonSize::Controller)
                                .on_press(move |_| on_b.call(stop)),
                            self.ring_visible && self.focus == stop,
                        ))
                    }),
            )
            .maybe_child(
                failure_message(&self.game.run).map(|m| TypeStyle::DeckMeta.text(m, t.danger)),
            )
    }
}
