//! The Install page: a form with an install location and two switches.
use reclaw_input::{FocusId, FocusNode};

use super::{DeckState, DeckView, Effect, Screen, TextField, ids, nodes::node};
use crate::{
    metrics::*,
    settings::{SettingsTarget, default_install_location},
    surface::row_height,
};

/// Values on the Install page that are plain data. The location text lives in `DeckApp` (a text
/// box needs a Freya state); everything else is here.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct InstallDraft {
    pub prerelease: bool,
}

impl DeckState {
    /// Fields stacked in the page body, then the footer buttons below them.
    pub(super) fn install_nodes(&self) -> Vec<FocusNode> {
        let gap = 8.;
        let (text_h, row_h) = (row_height(true, Density::Controller), row_height(false, Density::Controller));
        let mut y = 0.;
        let mut field = |id, h: f32| {
            let n = node(id, 0., y, SURFACE_MAX_W, h);
            y += h + gap;
            n
        };
        let mut nodes = vec![field(ids::INSTALL_LOCATION, text_h), field(ids::INSTALL_PRERELEASE, row_h)];
        nodes.push(node(ids::INSTALL_CANCEL, 0., y + 40., 200., DECK_TARGET_MIN));
        nodes.push(node(ids::INSTALL_SUBMIT, 216., y + 40., 200., DECK_TARGET_MIN));
        nodes
    }

    pub(super) fn open_install(&mut self, app: u32, view: &DeckView) {
        self.install = InstallDraft::default();
        self.go(Screen::Install(app), view);
    }

    pub(super) fn activate_install(&mut self, app: u32, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        match id {
            i if i == ids::INSTALL_LOCATION => self.begin_entry(TextField::InstallLocation, fx),
            i if i == ids::INSTALL_PRERELEASE => self.install.prerelease = !self.install.prerelease,
            i if i == ids::INSTALL_CANCEL => self.go_back(view),
            i if i == ids::INSTALL_SUBMIT => {
                fx.push(Effect::SubmitInstall(app));
                self.go_back(view);
            }
            _ => {}
        }
    }

    /// The text boxes a page just shown must start with: the install form with the Library default,
    /// the settings pages with what is stored. Handed to the host of the boxes (`DeckApp`) once per
    /// arrival, so text being typed is never replaced by a later change elsewhere.
    pub(super) fn seed_texts_for_screen(&mut self) {
        self.text_seeds.clear();
        match self.screen {
            Screen::Install(_) => self.text_seeds.push((TextField::InstallLocation, default_install_location(&self.values))),
            Screen::Settings(target) => {
                let fields: &[TextField] = match target {
                    SettingsTarget::Global => &[TextField::DefaultLocation],
                    SettingsTarget::App(_) => &[TextField::LaunchOptions, TextField::SdlOverride],
                };
                for &field in fields {
                    let stored = field.key().and_then(|key| self.values.text(target, key)).unwrap_or_default().to_string();
                    self.text_seeds.push((field, stored));
                }
            }
            _ => {}
        }
    }

    /// Take the boxes to fill since the last call.
    pub fn take_text_seeds(&mut self) -> Vec<(TextField, String)> {
        std::mem::take(&mut self.text_seeds)
    }
}
