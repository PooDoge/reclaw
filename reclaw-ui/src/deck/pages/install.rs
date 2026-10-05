use freya::prelude::*;
use reclaw_input::FocusId;

use crate::{
    deck::{FocusFrame, InstallDraft, TextBoxes, TextField, ids},
    metrics::*,
    prelude::*,
    surface::{RowControl, SettingRow},
};

/// The body of the Install page: where to put it, which of the user's own files to build from, and
/// two switches. The footer (Cancel, Install) is the page's, not this component's.
#[derive(Clone, PartialEq)]
pub struct InstallBody {
    draft: InstallDraft,
    texts: TextBoxes,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl InstallBody {
    pub fn new(draft: InstallDraft, texts: TextBoxes, focus: FocusId, ring_visible: bool, on_click: EventHandler<FocusId>) -> Self {
        Self { draft, texts, focus, ring_visible, on_click }
    }
}

impl Component for InstallBody {
    fn render(&self) -> impl IntoElement {
        let focused = |id: FocusId| self.ring_visible && self.focus == id;
        let press = |id: FocusId| {
            let on_click = self.on_click.clone();
            EventHandler::new(move |()| on_click.call(id))
        };
        let (input, a11y) = self.texts.get(TextField::InstallLocation);
        let file_text = self.draft.game_file.clone().unwrap_or_else(|| "Choose file".to_string());

        rect()
            .vertical()
            .spacing(8.)
            .width(Size::fill())
            .child(
                SettingRow::new(
                    "Install location",
                    RowControl::Text { input, placeholder: "Choose a folder".into(), a11y, secret: false },
                    Density::Controller,
                )
                .focused(focused(ids::INSTALL_LOCATION))
                .on_press(press(ids::INSTALL_LOCATION)),
            )
            .child(
                SettingRow::new("Your game file", RowControl::Value { text: file_text, opens_menu: false }, Density::Controller)
                    .description("Never downloaded for you; Reclaw builds from your copy.")
                    .focused(focused(ids::INSTALL_FILE))
                    .on_press(press(ids::INSTALL_FILE)),
            )
            .child(
                SettingRow::new("Create desktop shortcut", RowControl::Toggle(self.draft.shortcut), Density::Controller)
                    .focused(focused(ids::INSTALL_SHORTCUT))
                    .on_press(press(ids::INSTALL_SHORTCUT)),
            )
            .child(
                SettingRow::new("Keep pre-release builds", RowControl::Toggle(self.draft.prerelease), Density::Controller)
                    .focused(focused(ids::INSTALL_PRERELEASE))
                    .on_press(press(ids::INSTALL_PRERELEASE)),
            )
    }
}

/// Cancel and Install for the footer of the Install page. Install is disabled until a file is chosen.
pub fn install_footer(draft: &InstallDraft, focus: FocusId, ring_visible: bool, on_click: EventHandler<FocusId>) -> Element {
    let (a, b) = (on_click.clone(), on_click);
    rect()
        .horizontal()
        .spacing(SPACE_4)
        .child(FocusFrame::new(
            ActionButton::new(ButtonVariant::Secondary)
                .label("Cancel")
                .size(ButtonSize::Controller)
                .on_press(move |_| a.call(ids::INSTALL_CANCEL)),
            ring_visible && focus == ids::INSTALL_CANCEL,
        ))
        .child(FocusFrame::new(
            ActionButton::install()
                .icon(IconName::Download)
                .label("Install")
                .size(ButtonSize::Controller)
                .enabled(draft.can_submit())
                .on_press(move |_| b.call(ids::INSTALL_SUBMIT)),
            ring_visible && focus == ids::INSTALL_SUBMIT,
        ))
        .into_element()
}
