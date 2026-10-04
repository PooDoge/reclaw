//! The Install page: a form with the user's own game file, an install location, two switches.
use reclaw_input::{FocusId, FocusNode};

use super::{DeckState, DeckView, Effect, Screen, TextField, ids, nodes::node};
use crate::{metrics::*, surface::row_height};

/// Values on the Install page that are plain data. The location text lives in `DeckApp` (a text
/// box needs a Freya state); everything else is here.
#[derive(Clone, PartialEq, Debug)]
pub struct InstallDraft {
    pub game_file: Option<String>,
    pub shortcut: bool,
    pub prerelease: bool,
}

impl Default for InstallDraft {
    fn default() -> Self {
        Self { game_file: None, shortcut: true, prerelease: false }
    }
}

impl InstallDraft {
    /// Install is allowed once the user has chosen their own game file.
    pub fn can_submit(&self) -> bool {
        self.game_file.is_some()
    }
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
        let mut nodes = vec![
            field(ids::INSTALL_LOCATION, text_h),
            field(ids::INSTALL_FILE, row_h),
            field(ids::INSTALL_SHORTCUT, row_h),
            field(ids::INSTALL_PRERELEASE, row_h),
        ];
        nodes.push(node(ids::INSTALL_CANCEL, 0., y + 40., 200., DECK_TARGET_MIN));
        nodes.push(node(ids::INSTALL_SUBMIT, 216., y + 40., 200., DECK_TARGET_MIN));
        nodes
    }

    /// The host answered `Effect::ChooseFile`.
    pub fn set_install_file(&mut self, path: impl Into<String>) {
        self.install.game_file = Some(path.into());
    }

    pub(super) fn open_install(&mut self, app: u32, view: &DeckView) {
        self.install = InstallDraft::default();
        self.go(Screen::Install(app), view);
    }

    pub(super) fn activate_install(&mut self, app: u32, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        match id {
            i if i == ids::INSTALL_LOCATION => self.begin_entry(TextField::InstallLocation, fx),
            i if i == ids::INSTALL_FILE => fx.push(Effect::ChooseFile(app)),
            i if i == ids::INSTALL_SHORTCUT => self.install.shortcut = !self.install.shortcut,
            i if i == ids::INSTALL_PRERELEASE => self.install.prerelease = !self.install.prerelease,
            i if i == ids::INSTALL_CANCEL => self.go_back(view),
            i if i == ids::INSTALL_SUBMIT && self.install.can_submit() => {
                fx.push(Effect::SubmitInstall(app));
                self.go_back(view);
            }
            _ => {}
        }
    }
}
