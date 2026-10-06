use freya::prelude::*;
use reclaw_input::FocusId;

use crate::{
    deck::{FocusFrame, InstallDraft, TextBoxes, TextField, ids},
    metrics::*,
    prelude::*,
    settings::FALLBACK_LOCATION,
    surface::{RowControl, SettingRow},
};

/// The body of the Install page: where to put it, and two switches. The footer (Cancel, Install) is the page's, not this component's.
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

        rect()
            .vertical()
            .spacing(8.)
            .width(Size::fill())
            .child(
                SettingRow::new(
                    "Install location",
                    RowControl::Text { input, placeholder: FALLBACK_LOCATION.into(), a11y, secret: false },
                    Density::Controller,
                )
                .focused(focused(ids::INSTALL_LOCATION))
                .on_press(press(ids::INSTALL_LOCATION)),
            )
            .child(
                SettingRow::new("Keep pre-release builds", RowControl::Toggle(self.draft.prerelease), Density::Controller)
                    .focused(focused(ids::INSTALL_PRERELEASE))
                    .on_press(press(ids::INSTALL_PRERELEASE)),
            )
    }
}

/// Cancel and Install for the footer of the Install page.
pub fn install_footer(focus: FocusId, ring_visible: bool, on_click: EventHandler<FocusId>) -> Element {
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
                .on_press(move |_| b.call(ids::INSTALL_SUBMIT)),
            ring_visible && focus == ids::INSTALL_SUBMIT,
        ))
        .into_element()
}
