//! Dialogs the desktop opens over whichever page is showing: install, the Manage menu and the
//! uninstall confirmation. They live in the frame, above the stage, so a page transition never
//! tears one down and any page can open one through [`GameDialogs`].
//!
//! * `install`: the install form; `manage`: the anchored or centered Options menu; `confirm`: uninstall
//! * `slot`: one dialog's open state, which also takes a navigation layer so Back closes it
use freya::prelude::*;

mod confirm;
mod install;
mod manage;
mod picker;
mod slot;

pub use picker::OpenPicker;
pub(super) use slot::Slot;

use crate::{
    app_menu::{MenuAction, options_menu},
    model::GameEntry,
    surface::MenuState,
};

/// An open Manage menu and where it was pressed.
#[derive(Clone, PartialEq)]
pub(super) struct OpenManage {
    pub game: u32,
    pub menu: MenuState<MenuAction>,
    pub at: (f32, f32),
}

/// Opens the desktop's dialogs. `Copy`: take it from `use_desktop_ui()` and move it into handlers.
#[derive(Clone, Copy, PartialEq)]
pub struct GameDialogs {
    pub(super) install: Slot<u32>,
    pub(super) manage: Slot<OpenManage>,
    pub(super) confirm: Slot<u32>,
    pub(super) picker: Slot<OpenPicker>,
}

impl GameDialogs {
    /// Hooks: call once, from the frame.
    pub fn use_new() -> Self {
        Self { install: Slot::use_new(), manage: Slot::use_new(), confirm: Slot::use_new(), picker: Slot::use_new() }
    }

    /// The game whose install form is open, if any. Reading it subscribes the caller, and it reads
    /// `None` once the form is closed by any means, Back included.
    pub fn installing(&self) -> Option<u32> {
        self.install.get()
    }

    /// Open the install form for a game.
    pub fn install(&self, game: u32) {
        self.install.open(game);
    }

    /// Open the Options menu for a game at a point (where it was pressed).
    pub fn manage(&self, game: &GameEntry, at: (f32, f32)) {
        self.manage.open(OpenManage { game: game.id, menu: options_menu(game, true), at });
    }

    /// Open a list of options to choose one from. `on_pick` is called with the index chosen.
    pub fn pick(&self, picker: OpenPicker) {
        self.picker.open(picker);
    }

    /// Ask whether to uninstall a game. Nothing is sent before the answer.
    pub fn uninstall(&self, game: u32) {
        self.confirm.open(game);
    }
}

/// Renders whichever dialogs are open. Put it last in the frame so it draws over everything.
#[derive(Clone, Copy, PartialEq)]
pub struct GameDialogsLayer {
    pub dialogs: GameDialogs,
}

impl Component for GameDialogsLayer {
    fn render(&self) -> impl IntoElement {
        let d = self.dialogs;
        rect()
            .position(Position::new_absolute().top(0.).left(0.))
            .child(install::InstallSheet { open: d.install })
            .child(manage::ManageView { manage: d.manage, confirm: d.confirm })
            .child(picker::PickerView { picker: d.picker })
            .child(confirm::UninstallConfirm { confirm: d.confirm })
    }
}
