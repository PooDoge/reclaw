use freya::prelude::*;
use reclaw_input::FocusId;

use super::focus::FocusFrame;
use crate::{deck::ids, metrics::*, prelude::*, typography::TypeStyle};

/// A short question over a darkened screen: title, one sentence, Cancel and the action. Cancel is
/// where focus starts, so the destructive choice is never one stray press away.
#[derive(Clone, PartialEq)]
pub struct ConfirmOverlay {
    copy: ConfirmCopy,
    focus: FocusId,
    ring_visible: bool,
    window: (f32, f32),
    on_click: EventHandler<FocusId>,
    on_dismiss: EventHandler<()>,
}

/// The words of a confirmation.
#[derive(Clone, PartialEq)]
pub struct ConfirmCopy {
    pub title: String,
    pub message: &'static str,
    pub action_label: &'static str,
}

impl ConfirmOverlay {
    pub fn new(
        copy: ConfirmCopy,
        focus: FocusId,
        ring_visible: bool,
        window: (f32, f32),
        on_click: EventHandler<FocusId>,
        on_dismiss: EventHandler<()>,
    ) -> Self {
        Self { copy, focus, ring_visible, window, on_click, on_dismiss }
    }
}

impl Component for ConfirmOverlay {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (w, h) = self.window;
        let (a, b, dismiss) = (self.on_click.clone(), self.on_click.clone(), self.on_dismiss.clone());
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
                    .background(t.deck_dim)
                    .on_press(move |_| dismiss.call(())),
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
                            .spacing(SPACE_4)
                            .width(Size::px((w - 48.).min(480.)))
                            .padding(SPACE_5)
                            .background(t.bg_panel)
                            .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
                            .corner_radius(RADIUS_LG)
                            .interactive(Interactive::Yes)
                            .child(TypeStyle::DeckHeading.text(self.copy.title.clone(), t.ink))
                            .child(TypeStyle::DeckBody.text(self.copy.message, t.ink_muted).max_lines(4))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(SPACE_4)
                                    .padding(Gaps::new(SPACE_2, 0., 0., 0.))
                                    .child(FocusFrame::new(
                                        ActionButton::new(ButtonVariant::Secondary)
                                            .label("Cancel")
                                            .size(ButtonSize::Controller)
                                            .on_press(move |_| a.call(ids::CONFIRM_CANCEL)),
                                        self.ring_visible && self.focus == ids::CONFIRM_CANCEL,
                                    ))
                                    .child(FocusFrame::new(
                                        ActionButton::new(ButtonVariant::Danger)
                                            .label(self.copy.action_label)
                                            .size(ButtonSize::Controller)
                                            .on_press(move |_| b.call(ids::CONFIRM_OK)),
                                        self.ring_visible && self.focus == ids::CONFIRM_OK,
                                    )),
                            ),
                    ),
            )
    }
}
