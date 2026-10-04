use std::time::Duration;

use freya::{
    animation::*,
    engine::prelude::{Color as SkColor, Paint, PaintCap, PaintStyle, SkRect},
    prelude::*,
};
use reclaw_input::{Button, ControllerKind, FocusId, GlyphFace};

use super::{focus::FocusFrame, glyph::ButtonGlyph};
use crate::{
    deck::{LastInput, ids},
    metrics::*,
    notices::{DETAILS_BUTTON, DISMISS_ALL_BUTTON, HoldAction, Notice, NoticeKind},
    prelude::*,
    typography::TypeStyle,
};

/// The icon and color that mean each kind of notification.
fn notice_look(t: &Reclaw, kind: NoticeKind) -> (IconName, Color) {
    match kind {
        NoticeKind::UpdateAvailable => (IconName::Download, t.warn),
        NoticeKind::UpdateFinished | NoticeKind::InstallFinished | NoticeKind::ModInstalled => (IconName::Check, t.ok),
        NoticeKind::DownloadFailed => (IconName::Alert, t.danger),
    }
}

/// A circular progress ring drawn around the button glyph while the button is held: a faint track,
/// and an arc from the top that sweeps clockwise as the hold nears its end.
///
/// The canvas does not repaint on its own when only its closure changes (the callbacks always compare
/// equal), so it is keyed by the progress, in steps fine enough to look smooth.
fn ring(t: &Reclaw, diameter: f32, progress: f32) -> Element {
    let (track, fill) = (t.line_strong, t.accent);
    let steps = (progress.clamp(0., 1.) * 120.).round() as u32;
    canvas(RenderCallback::new(move |ctx| {
        let stroke = 4.0_f32;
        let inset = stroke / 2.;
        let oval = SkRect::from_xywh(inset, inset, ctx.size.width - stroke, ctx.size.height - stroke);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Stroke);
        paint.set_stroke_width(stroke);
        paint.set_color(SkColor::from(track));
        ctx.canvas.draw_oval(oval, &paint);
        if steps > 0 {
            paint.set_color(SkColor::from(fill));
            paint.set_stroke_cap(PaintCap::Round);
            ctx.canvas.draw_arc(oval, -90., 360. * (steps as f32 / 120.), false, &paint);
        }
    }))
    .width(Size::px(diameter))
    .height(Size::px(diameter))
    .key(steps)
    .into_element()
}

/// One hold prompt: the button's glyph inside a ring that fills while it is held, and what the hold
/// does. The ring runs for the hold's length on its own clock; the pad reader's clock decides when it
/// really completes, and the two agree to within a frame.
#[derive(Clone, PartialEq)]
struct HoldPrompt {
    action: HoldAction,
    label: &'static str,
    /// The button is down right now.
    holding: bool,
    glyph: Option<GlyphFace>,
    keycap: &'static str,
}

impl Component for HoldPrompt {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let after = self.action.after();
        let animation = use_animation_with_dependencies(&self.holding, move |conf, _| {
            conf.on_change(OnChange::Rerun);
            AnimNum::new(0., 1.).duration(Duration::from_millis(after.as_millis() as u64)).function(Function::Linear)
        });
        let progress = if self.holding { animation.get().value() } else { 0. };
        let glyph = match self.glyph {
            Some(face) => ButtonGlyph::new(face),
            None => ButtonGlyph::keycap(self.keycap),
        };
        let diameter = 44.;
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(SPACE_2)
            .child(
                rect()
                    .width(Size::px(diameter))
                    .height(Size::px(diameter))
                    .center()
                    .child(rect().position(Position::new_absolute().top(0.).left(0.)).child(ring(&t, diameter, progress)))
                    .child(glyph),
            )
            .child(TypeStyle::DeckHint.text(self.label, if self.holding { t.ink } else { t.ink_muted }).max_lines(1))
    }
}

/// The newest notification as a card at the bottom right: what happened, and the two holds that act
/// on it. Not focusable: it never takes the pad from the page; the holds are the only way in.
#[derive(Clone, PartialEq)]
pub struct NoticeToast {
    notice: Notice,
    /// The button being held, for the ring.
    holding: Option<Button>,
    kind: ControllerKind,
    last_input: LastInput,
    window: (f32, f32),
}

pub const TOAST_W: f32 = 460.;
pub const TOAST_H: f32 = 132.;

impl NoticeToast {
    pub fn new(notice: Notice, holding: Option<Button>, kind: ControllerKind, last_input: LastInput, window: (f32, f32)) -> Self {
        Self { notice, holding, kind, last_input, window }
    }
}

impl Component for NoticeToast {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (w, h) = self.window;
        let (icon_name, color) = notice_look(&t, self.notice.kind);

        // Slides in from the right when a new notice arrives.
        let id = self.notice.id;
        let entrance = use_animation_with_dependencies(&id, |conf, _| {
            conf.on_creation(OnCreation::Run);
            conf.on_change(OnChange::Rerun);
            AnimNum::new(0., 1.).duration(Duration::from_millis(280)).function(Function::Expo).ease(Ease::Out)
        });
        let p = entrance.get().value();

        let prompt = |action: HoldAction, label: &'static str, keycap: &'static str| {
            let glyph = matches!(self.last_input, LastInput::Gamepad(_)).then(|| self.kind.glyph(action.button()));
            HoldPrompt { action, label, holding: self.holding == Some(action.button()), glyph, keycap }.into_element()
        };
        debug_assert!(DETAILS_BUTTON != DISMISS_ALL_BUTTON);

        let left = (w - DECK_SAFE_X - TOAST_W).max(0.);
        let top = (h - DECK_SAFE_Y - DECK_HINT_H - TOAST_H - SPACE_3).max(0.);
        rect()
            .position(Position::new_absolute().top(top).left(left))
            .layer(Layer::Overlay)
            .width(Size::px(TOAST_W))
            .height(Size::px(TOAST_H))
            .opacity(p)
            .offset_x((1. - p) * 48.)
            .interactive(Interactive::No)
            .vertical()
            .spacing(SPACE_3)
            .padding(SPACE_4)
            .background(t.bg_panel)
            .border(
                Border::new().fill(color).width(BorderWidth { left: 4., top: 1., right: 1., bottom: 1. }).alignment(BorderAlignment::Inner),
            )
            .corner_radius(RADIUS_LG)
            .child(
                rect().horizontal().cross_align(Alignment::Center).spacing(SPACE_3).child(icon(icon_name, 28., color)).child(
                    rect()
                        .vertical()
                        .width(Size::flex(1.))
                        .child(
                            TypeStyle::DeckLabel.text(self.notice.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis),
                        )
                        .child(
                            TypeStyle::DeckMeta
                                .text(self.notice.body.clone(), t.ink_muted)
                                .max_lines(1)
                                .text_overflow(TextOverflow::Ellipsis),
                        ),
                ),
            )
            .child(rect().horizontal().spacing(SPACE_5).child(prompt(HoldAction::Details, "Hold for details", "X")).child(prompt(
                HoldAction::DismissAll,
                "Hold to dismiss all",
                "Y",
            )))
    }
}

/// What a notification says in full: the release notes or the reason, with Close and Dismiss.
#[derive(Clone, PartialEq)]
pub struct NoticeDetails {
    notice: Notice,
    focus: FocusId,
    ring_visible: bool,
    window: (f32, f32),
    on_click: EventHandler<FocusId>,
    on_dismiss: EventHandler<()>,
}

impl NoticeDetails {
    pub fn new(
        notice: Notice,
        focus: FocusId,
        ring_visible: bool,
        window: (f32, f32),
        on_click: EventHandler<FocusId>,
        on_dismiss: EventHandler<()>,
    ) -> Self {
        Self { notice, focus, ring_visible, window, on_click, on_dismiss }
    }
}

impl Component for NoticeDetails {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (w, h) = self.window;
        let (close, dismiss_notice, scrim) = (self.on_click.clone(), self.on_click.clone(), self.on_dismiss.clone());
        let lines: Vec<Element> = self
            .notice
            .details
            .iter()
            .take(10)
            .map(|line| {
                rect()
                    .horizontal()
                    .spacing(SPACE_2)
                    .child(TypeStyle::DeckBody.text("•", t.ink_subtle))
                    .child(rect().width(Size::flex(1.)).child(TypeStyle::DeckBody.text(line.clone(), t.ink).max_lines(3)))
                    .into_element()
            })
            .collect();
        let more = self.notice.details.len().saturating_sub(10);
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
                    .background(Color::from_argb(217, 5, 8, 12))
                    .on_press(move |_| scrim.call(())),
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
                            .width(Size::px((w - 48.).min(560.)))
                            .padding(SPACE_5)
                            .background(t.bg_panel)
                            .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
                            .corner_radius(RADIUS_LG)
                            .interactive(Interactive::Yes)
                            .child(TypeStyle::DeckHeading.text(self.notice.title.clone(), t.ink))
                            .child(TypeStyle::DeckMeta.text(self.notice.body.clone(), t.ink_muted))
                            .maybe(!lines.is_empty(), |el| el.child(rect().vertical().spacing(SPACE_2).children(lines)))
                            .maybe(more > 0, |el| el.child(TypeStyle::DeckMeta.text(format!("and {more} more"), t.ink_subtle)))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(SPACE_4)
                                    .padding(Gaps::new(SPACE_2, 0., 0., 0.))
                                    .child(FocusFrame::new(
                                        ActionButton::new(ButtonVariant::Secondary)
                                            .label("Close")
                                            .size(ButtonSize::Controller)
                                            .on_press(move |_| close.call(ids::NOTICE_CLOSE)),
                                        self.ring_visible && self.focus == ids::NOTICE_CLOSE,
                                    ))
                                    .child(FocusFrame::new(
                                        ActionButton::new(ButtonVariant::Primary)
                                            .label("Dismiss")
                                            .size(ButtonSize::Controller)
                                            .on_press(move |_| dismiss_notice.call(ids::NOTICE_DISMISS)),
                                        self.ring_visible && self.focus == ids::NOTICE_DISMISS,
                                    )),
                            ),
                    ),
            )
    }
}
