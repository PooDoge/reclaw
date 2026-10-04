use std::time::SystemTime;

use freya::prelude::*;
use reclaw_input::FocusId;
use reclaw_runtime::RunState;

use crate::deck::{BANNER_H, FocusFrame};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// "Running for 12 min" and so on. Whole minutes: a launcher does not need a ticking clock.
pub fn elapsed_label(run: &RunState) -> String {
    match run {
        RunState::Running { since, .. } => {
            let mins = SystemTime::now().duration_since(*since).map(|d| d.as_secs() / 60).unwrap_or(0);
            if mins == 0 { "Running, just started".into() } else { format!("Running for {mins} min") }
        }
        RunState::Stopping { .. } => "Stopping".into(),
        RunState::Starting => "Starting".into(),
        _ => String::new(),
    }
}

/// Top of Deck Home while an app is active. Resume and Stop are both focus targets.
#[derive(Clone, PartialEq)]
pub struct NowPlayingBanner {
    game: GameEntry,
    focus: FocusId,
    ring_visible: bool,
    resume_id: FocusId,
    stop_id: FocusId,
    on_click: EventHandler<FocusId>,
}

impl NowPlayingBanner {
    pub fn new(
        game: GameEntry,
        focus: FocusId,
        ring_visible: bool,
        resume_id: FocusId,
        stop_id: FocusId,
        on_click: EventHandler<FocusId>,
    ) -> Self {
        Self { game, focus, ring_visible, resume_id, stop_id, on_click }
    }
}

impl Component for NowPlayingBanner {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let stopping = matches!(self.game.run, RunState::Stopping { .. });
        let (resume, stop) = (self.resume_id, self.stop_id);
        let (on_a, on_b) = (self.on_click.clone(), self.on_click.clone());

        rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_4)
            .width(Size::fill())
            .height(Size::px(BANNER_H))
            .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
            .background(t.bg_panel)
            .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
            .corner_radius(RADIUS_MD)
            .child(StatusBadge::running())
            .child(
                rect()
                    .vertical()
                    .width(Size::flex(1.))
                    .child(TypeStyle::DeckLabel.text(self.game.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis))
                    .child(TypeStyle::DeckMeta.text(elapsed_label(&self.game.run), t.ink_muted)),
            )
            .child(FocusFrame::new(
                ActionButton::install()
                    .icon(IconName::Play)
                    .label("Resume")
                    .size(ButtonSize::Controller)
                    .enabled(!stopping)
                    .on_press(move |_| on_a.call(resume)),
                self.ring_visible && self.focus == resume,
            ))
            .child(FocusFrame::new(
                ActionButton::new(ButtonVariant::Danger)
                    .icon(IconName::Stop)
                    .label(if stopping { "Force quit" } else { "Stop" })
                    .size(ButtonSize::Controller)
                    .on_press(move |_| on_b.call(stop)),
                self.ring_visible && self.focus == stop,
            ))
    }
}
