use freya::prelude::*;

use super::{command::WindowCommand, platform::use_maximized};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// The height of the custom title bar.
pub const TITLEBAR_H: f32 = 36.;
const BUTTON_W: f32 = 46.;

/// One of the three window buttons. Close goes red under the pointer, like every desktop's.
#[derive(Clone, PartialEq)]
struct WindowButton {
    glyph: IconName,
    danger: bool,
    on_press: EventHandler<()>,
}

impl Component for WindowButton {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let mut hovering = use_state(|| false);
        let on_press = self.on_press.clone();
        let (fill, ink) = match (hovering(), self.danger) {
            (true, true) => (t.danger, t.on_accent),
            (true, false) => (t.bg_raised, t.ink),
            (false, _) => (CLEAR, t.ink_muted),
        };
        rect()
            .width(Size::px(BUTTON_W))
            .height(Size::px(TITLEBAR_H))
            .center()
            .background(fill)
            .on_pointer_enter(move |_| hovering.set(true))
            .on_pointer_leave(move |_| hovering.set(false))
            .on_press(move |_| on_press.call(()))
            .child(icon(self.glyph, 14., ink))
    }
}

/// The custom title bar: the app's name on a drag area (drag to move, double-press to maximize),
/// and minimize, maximize or restore, and close at the right. Resize bands along the window's edges
/// come from the borderless plugin.
#[derive(Clone, PartialEq)]
pub struct Titlebar {
    on_command: EventHandler<WindowCommand>,
    /// A real window is behind the bar, so it can ask whether that window is maximized.
    attached: bool,
    /// The page being shown, so a window list or a glance at the corner says where the app is.
    page: &'static str,
}

impl Titlebar {
    pub fn new(on_command: EventHandler<WindowCommand>, attached: bool, page: &'static str) -> Self {
        Self { on_command, attached, page }
    }
}

impl Component for Titlebar {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let maximized = use_maximized(self.attached);
        let send = |command: WindowCommand| {
            let on_command = self.on_command.clone();
            EventHandler::new(move |()| on_command.call(command.clone()))
        };
        let button = |glyph, danger, command| WindowButton { glyph, danger, on_press: send(command) };

        rect()
            .vertical()
            .width(Size::fill())
            .child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .width(Size::fill())
                    .height(Size::px(TITLEBAR_H))
                    .background(t.bg_deep)
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(SPACE_2)
                            .width(Size::flex(1.))
                            .height(Size::fill())
                            .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
                            .window_drag()
                            .child(icon(IconName::Gamepad, 16., t.accent))
                            .child(TypeStyle::Meta.text(self.page, t.ink_muted)),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .child(button(IconName::Minus, false, WindowCommand::Minimize))
                            .child(button(
                                if maximized() { IconName::Restore } else { IconName::Square },
                                false,
                                WindowCommand::ToggleMaximize,
                            ))
                            .child(button(IconName::X, true, WindowCommand::Close)),
                    ),
            )
            .child(rect().width(Size::fill()).height(Size::px(1.)).background(t.line))
    }
}
