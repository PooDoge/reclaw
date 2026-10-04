use freya::prelude::*;

use super::ToggleSwitch;
use crate::{
    metrics::*,
    prelude::*,
    surface::{Dialog, DialogAction, Presentation, SurfaceContext, SurfaceKind, presentation},
    typography::TypeStyle,
};

/// Space between a field's label and the field.
const LABEL_GAP: f32 = 6.;
/// Height of a field label, for working out where the location field sits in the body.
const LABEL_H: f32 = 20.;

/// The install form, shown from the hero's Install. One component for every form factor: a
/// popup on desktop, a full-screen page with Back on phones, short touch screens and in Deck mode
/// (see [`presentation`]). It renders only while `open`, which lets the popup run its close animation.
///
/// Install stays disabled until the user has chosen their own game file. The field says the file
/// is never downloaded for you. While the on-screen keyboard is up (`keyboard_inset`) the full
/// screen page hides its footer and scrolls the location field into view.
#[derive(Clone, PartialEq)]
pub struct InstallDialog {
    open: bool,
    title: String,
    location: State<String>,
    game_file: State<Option<String>>,
    shortcut: State<bool>,
    prerelease: State<bool>,
    surface: SurfaceContext,
    window: (f32, f32),
    keyboard_inset: f32,
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
            surface: SurfaceContext::new(LayoutClass::Wide, Density::Pointer, 800.),
            window: (1100., 800.),
            keyboard_inset: 0.,
            on_choose_file: None,
            on_confirm: None,
            on_cancel: None,
        }
    }

    /// The form factor, which picks popup or full screen.
    pub fn surface(mut self, surface: SurfaceContext) -> Self {
        self.surface = surface;
        self
    }

    pub fn window(mut self, window: (f32, f32)) -> Self {
        self.window = window;
        self
    }

    pub fn keyboard_inset(mut self, inset: f32) -> Self {
        self.keyboard_inset = inset;
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

    fn body(&self, t: Reclaw, density: Density) -> Rect {
        let button_size = ButtonSize::for_density(density, false);
        let file = self.game_file.read().clone();
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
                    .border(Border::new().fill(t.line_strong).width(1.).alignment(BorderAlignment::Inner))
                    .corner_radius(RADIUS_MD)
                    .child(icon(IconName::File, 16., t.ink_subtle))
                    .child(match file {
                        Some(path) => TypeStyle::Mono.text(path, t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis),
                        None => TypeStyle::Body.text("Choose the file you own", t.ink_subtle),
                    }),
            )
            .child(
                ActionButton::new(ButtonVariant::Secondary)
                    .label("Browse")
                    .size(button_size)
                    .map(self.on_choose_file.clone(), |b, h| b.on_press(move |_| h.call(()))),
            );

        let toggle_row = |text: &'static str, state: State<bool>| {
            rect()
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .width(Size::fill())
                .height(Size::px(density.row_height()))
                .child(rect().width(Size::flex(1.)).child(TypeStyle::Body.text(text, t.ink)))
                .child(ToggleSwitch::new(state))
        };

        rect()
            .vertical()
            .spacing(SPACE_4)
            .width(Size::fill())
            .child(
                rect()
                    .vertical()
                    .spacing(LABEL_GAP)
                    .width(Size::fill())
                    .child(field_label("Install location"))
                    .child(SearchField::new(self.location).icon(IconName::Folder).placeholder("Choose a folder").density(density)),
            )
            .child(
                rect()
                    .vertical()
                    .spacing(LABEL_GAP)
                    .width(Size::fill())
                    .child(field_label("Your game file (never downloaded for you)"))
                    .child(file_row),
            )
            .child(toggle_row("Create desktop shortcut", self.shortcut))
            .child(toggle_row("Keep pre-release builds", self.prerelease))
    }
}

impl Component for InstallDialog {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let shown = presentation(SurfaceKind::Form, self.surface);
        if !self.open {
            // The popup animates out only if it stays mounted; the other forms just disappear.
            return if shown == Presentation::Popup { Popup::new().into_element() } else { rect().into_element() };
        }

        let density = self.surface.density;
        let on_cancel = self.on_cancel.clone().unwrap_or_else(|| EventHandler::new(|()| {}));
        let on_confirm = self.on_confirm.clone().unwrap_or_else(|| EventHandler::new(|()| {}));
        let actions = vec![
            DialogAction::new("Cancel", ButtonVariant::Ghost, on_cancel.clone()),
            DialogAction::new("Install", ButtonVariant::Install, on_confirm)
                .icon(IconName::Download)
                .enabled(self.game_file.read().is_some()),
        ];
        // The location field is the only text input; keep it above the keyboard.
        let reveal = (self.keyboard_inset > 0.).then(|| (0., LABEL_H + LABEL_GAP + density.button_height()));

        Dialog::new(SurfaceKind::Form, self.surface, self.window, self.title.clone(), self.body(t, density), actions, on_cancel)
            .keyboard_inset(self.keyboard_inset)
            .reveal(reveal)
            .into_element()
    }
}
