use freya::prelude::*;

use super::Slot;
use crate::{
    app_menu::MenuAction,
    desktop::use_desktop_ui,
    prelude::*,
    shell::use_shell,
    store::use_games,
    surface::{Dialog, DialogAction, SurfaceKind},
    typography::TypeStyle,
};

/// "Uninstall X?": a popup on desktop, a small centered card on touch.
#[derive(Clone, PartialEq)]
pub(super) struct UninstallConfirm {
    pub confirm: Slot<u32>,
}

impl Component for UninstallConfirm {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui) = (use_shell(), use_desktop_ui());
        let env = *ui.env.read();
        let games = use_games();
        let confirm = self.confirm;
        let Some(id) = confirm.get() else { return rect().into_element() };
        let Some(game) = games.iter().find(|g| g.id == id) else { return rect().into_element() };

        let on_effect = shell.on_effect.clone();
        let cancel = EventHandler::new(move |()| confirm.close());
        let accept = EventHandler::new(move |()| {
            confirm.close();
            if let Some(effect) = MenuAction::Uninstall.confirmed_effect(id) {
                on_effect.call(effect);
            }
        });
        let body = TypeStyle::Body.text("Removes it from this device. Your own game file is never touched.", t.ink_muted);
        let actions = vec![
            DialogAction::new("Cancel", ButtonVariant::Ghost, cancel.clone()),
            DialogAction::new("Uninstall", ButtonVariant::Danger, accept),
        ];
        Dialog::new(SurfaceKind::Confirm, env.surface(), env.window, format!("Uninstall {}?", game.title), body, actions, cancel)
            .into_element()
    }
}
