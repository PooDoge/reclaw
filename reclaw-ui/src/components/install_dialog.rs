use freya::prelude::*;

use super::ToggleSwitch;
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Modal shown from the hero's Install. A Freya `Popup`; it renders only while `open`, which is
/// what lets the Popup run its close animation.
///
/// Install stays disabled until the user has chosen their own game file. The field says the file
/// is never downloaded for you. Pass `touch` for 48px controls; a true bottom sheet needs a
/// custom overlay (Popup is always centered), which this does not provide.
#[derive(Clone, PartialEq)]
pub struct InstallDialog {
    open: bool,
    title: String,
    location: State<String>,
    game_file: State<Option<String>>,
    shortcut: State<bool>,
    prerelease: State<bool>,
    touch: bool,
    on_choose_file: Option<EventHandler<()>>,
    on_confirm: Option<EventHandler<()>>,
    on_cancel: Option<EventHandler<()>>,
}

impl InstallDialog {
    pub fn new(
        open: bool,
        game_title: &str,
        location: State<String>,
        game_file: State<Option<String>>,
        shortcut: State<bool>,
        prerelease: State<bool>,
    ) -> Self {
        Self {
            open,
            title: format!("Install {game_title}"),
            location,
            game_file,
            shortcut,
            prerelease,
            touch: false,
            on_choose_file: None,
            on_confirm: None,
            on_cancel: None,
        }
    }

    pub fn touch(mut self, touch: bool) -> Self {
        self.touch = touch;
        self
    }

    pub fn on_choose_file(mut self, handler: impl Into<EventHandler<()>>) -> Self {
        self.on_choose_file = Some(handler.into());
        self
    }

    pub fn on_confirm(mut self, handler: impl Into<EventHandler<()>>) -> Self {
        self.on_confirm = Some(handler.into());
        self
    }

    pub fn on_cancel(mut self, handler: impl Into<EventHandler<()>>) -> Self {
        self.on_cancel = Some(handler.into());
        self
    }
}

impl Component for InstallDialog {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        if !self.open {
            return Popup::new().into_element();
        }

        let density = if self.touch {
            Density::Touch
        } else {
            Density::Pointer
        };
        let button_size = if self.touch {
            ButtonSize::Touch
        } else {
            ButtonSize::Md
        };
        let file = self.game_file.read().clone();
        let has_file = file.is_some();

        let field_label = |text: &'static str| TypeStyle::Label.text(text, t.ink_muted);

        let file_row = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(
                rect()
                    .width(Size::flex(1.))
                    .height(Size::px(density.button_height()))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_2)
                    .padding(Gaps::new(0., SPACE_3, 0., SPACE_3))
                    .background(t.bg_raised)
                    .border(
                        Border::new()
                            .fill(t.line_strong)
                            .width(1.)
                            .alignment(BorderAlignment::Inner),
                    )
                    .corner_radius(RADIUS_MD)
                    .child(icon(IconName::File, 16., t.ink_subtle))
                    .child(match file {
                        Some(path) => TypeStyle::Mono
                            .text(path, t.ink)
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                        None => TypeStyle::Body.text("Choose the file you own", t.ink_subtle),
                    }),
            )
            .child(
                ActionButton::new(ButtonVariant::Secondary)
                    .label("Browse")
                    .size(button_size)
                    .map(self.on_choose_file.clone(), |b, h| {
                        b.on_press(move |_| h.call(()))
                    }),
            );

        let toggle_row = |text: &'static str, state: State<bool>| {
            rect()
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .width(Size::fill())
                .height(Size::px(density.row_height()))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(TypeStyle::Body.text(text, t.ink)),
                )
                .child(ToggleSwitch::new(state))
        };

        let content = rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::fill())
            .child(
                rect()
                    .vertical()
                    .spacing(6.)
                    .width(Size::fill())
                    .child(field_label("Install location"))
                    .child(
                        SearchField::new(self.location)
                            .placeholder("Choose a folder")
                            .density(density),
                    ),
            )
            .child(
                rect()
                    .vertical()
                    .spacing(6.)
                    .width(Size::fill())
                    .child(field_label("Your game file (never downloaded for you)"))
                    .child(file_row),
            )
            .child(toggle_row("Create desktop shortcut", self.shortcut))
            .child(toggle_row("Keep pre-release builds", self.prerelease));

        let cancel = self.on_cancel.clone();
        let confirm = self.on_confirm.clone();
        let close = self.on_cancel.clone();

        Popup::new()
            .background(t.bg_panel)
            .color(t.ink)
            .width(Size::px(420.))
            .on_close_request(move |_| {
                if let Some(h) = &close {
                    h.call(());
                }
            })
            .child(PopupTitle::new(self.title.clone()))
            .child(PopupContent::new().child(content))
            .child(
                PopupButtons::new()
                    .child(
                        ActionButton::new(ButtonVariant::Ghost)
                            .label("Cancel")
                            .size(button_size)
                            .map(cancel, |b, h| b.on_press(move |_| h.call(()))),
                    )
                    .child(
                        ActionButton::install()
                            .icon(IconName::Download)
                            .label("Install")
                            .size(button_size)
                            .enabled(has_file)
                            .map(confirm, |b, h| b.on_press(move |_| h.call(()))),
                    ),
            )
            .into_element()
    }
}
