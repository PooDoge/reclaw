use std::borrow::Cow;

use freya::prelude::*;

use super::{
    page::FullScreenPage,
    presentation::{Presentation, SurfaceContext, SurfaceKind, presentation},
};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

#[derive(Clone, PartialEq)]
pub struct DialogAction {
    pub label: Cow<'static, str>,
    pub variant: ButtonVariant,
    pub icon: Option<IconName>,
    pub enabled: bool,
    pub on_press: EventHandler<()>,
}

impl DialogAction {
    pub fn new(label: impl Into<Cow<'static, str>>, variant: ButtonVariant, on_press: impl Into<EventHandler<()>>) -> Self {
        Self { label: label.into(), variant, icon: None, enabled: true, on_press: on_press.into() }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn button(&self, size: ButtonSize) -> ActionButton {
        let on_press = self.on_press.clone();
        let mut button =
            ActionButton::new(self.variant).label(self.label.clone()).size(size).enabled(self.enabled).on_press(move |_| on_press.call(()));
        if let Some(icon) = self.icon {
            button = button.icon(icon);
        }
        button
    }
}

/// One dialog, presented the way the form factor wants (see [`presentation`]): a popup on desktop,
/// a full-screen page with Back on phones, handhelds and Deck mode, a small centered card for
/// confirmations. The caller supplies content and actions once.
#[derive(Clone, PartialEq)]
pub struct Dialog {
    kind: SurfaceKind,
    ctx: SurfaceContext,
    window: (f32, f32),
    keyboard_inset: f32,
    reveal: Option<(f32, f32)>,
    wide: bool,
    title: String,
    body: Element,
    actions: Vec<DialogAction>,
    on_close: EventHandler<()>,
}

impl Dialog {
    pub fn new(
        kind: SurfaceKind,
        ctx: SurfaceContext,
        window: (f32, f32),
        title: impl Into<String>,
        body: impl IntoElement,
        actions: Vec<DialogAction>,
        on_close: EventHandler<()>,
    ) -> Self {
        Self {
            kind,
            ctx,
            window,
            keyboard_inset: 0.,
            reveal: None,
            wide: false,
            title: title.into(),
            body: body.into_element(),
            actions,
            on_close,
        }
    }

    pub fn keyboard_inset(mut self, inset: f32) -> Self {
        self.keyboard_inset = inset;
        self
    }

    /// The focused field's top and bottom in body coordinates, to keep it above the keyboard.
    pub fn reveal(mut self, target: Option<(f32, f32)>) -> Self {
        self.reveal = target;
        self
    }

    /// A popup wide enough for lines of a log or a table. Other presentations already take the screen's width.
    pub fn wide(mut self, wide: bool) -> Self {
        self.wide = wide;
        self
    }

    pub fn presentation(&self) -> Presentation {
        presentation(self.kind, self.ctx)
    }
}

fn overlay_root(window: (f32, f32)) -> Rect {
    rect().position(Position::new_absolute().top(0.).left(0.)).layer(Layer::Overlay).width(Size::px(window.0)).height(Size::px(window.1))
}

impl Component for Dialog {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let size = ButtonSize::for_density(self.ctx.density, false);
        match self.presentation() {
            Presentation::FullScreen => {
                let on_close = self.on_close.clone();
                let footer = rect()
                    .horizontal()
                    .spacing(SPACE_3)
                    .children(self.actions.iter().map(|a| a.button(size.max(ButtonSize::Touch)).into_element()));
                overlay_root(self.window)
                    .child(
                        FullScreenPage::new(
                            self.title.clone(),
                            self.window,
                            self.ctx.density,
                            EventHandler::new(move |()| on_close.call(())),
                            self.body.clone(),
                        )
                        .keyboard_inset(self.keyboard_inset)
                        .reveal(self.reveal)
                        .footer(footer),
                    )
                    .into_element()
            }
            Presentation::Centered => {
                let (w, h) = self.window;
                let on_close = self.on_close.clone();
                overlay_root(self.window)
                    .child(
                        rect()
                            .position(Position::new_absolute().top(0.).left(0.))
                            .width(Size::px(w))
                            .height(Size::px(h))
                            .background(t.scrim)
                            .on_press(move |_| on_close.call(())),
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
                                    .width(Size::px((w - 48.).min(420.)))
                                    .padding(SPACE_5)
                                    .background(t.bg_panel)
                                    .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
                                    .corner_radius(RADIUS_LG)
                                    .interactive(Interactive::Yes)
                                    .child(TypeStyle::DeckHeading.text(self.title.clone(), t.ink))
                                    .child(self.body.clone())
                                    .child(
                                        rect()
                                            .horizontal()
                                            .main_align(Alignment::End)
                                            .spacing(SPACE_3)
                                            .children(self.actions.iter().map(|a| a.button(size).into_element())),
                                    ),
                            ),
                    )
                    .into_element()
            }
            Presentation::Popup | Presentation::Anchored => {
                let on_close = self.on_close.clone();
                Popup::new()
                    .background(t.bg_panel)
                    .color(t.ink)
                    .width(Size::px(match (self.wide, self.kind) {
                        (true, _) => 760.,
                        (false, SurfaceKind::Confirm) => 420.,
                        (false, _) => 480.,
                    }))
                    .on_close_request(move |_| on_close.call(()))
                    .child(PopupTitle::new(self.title.clone()))
                    .child(PopupContent::new().child(self.body.clone()))
                    .child(PopupButtons::new().children(self.actions.iter().map(|a| a.button(size).into_element())))
                    .into_element()
            }
        }
    }
}
