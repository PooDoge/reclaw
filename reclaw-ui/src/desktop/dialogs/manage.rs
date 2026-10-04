use freya::prelude::*;

use super::{OpenManage, Slot};
use crate::{
    app_menu::MenuAction,
    desktop::use_desktop_ui,
    nav::{Route, use_nav},
    shell::use_shell,
    surface::{MenuLevelView, MenuPlacement, ModalMenu, Outcome, Presentation, SurfaceKind, presentation},
};

/// The Options menu: a popover at the press point under a pointer, centered over a darkened screen
/// on touch.
#[derive(Clone, PartialEq)]
pub(super) struct ManageView {
    pub manage: Slot<OpenManage>,
    pub confirm: Slot<u32>,
}

impl Component for ManageView {
    fn render(&self) -> impl IntoElement {
        let (shell, ui, nav) = (use_shell(), use_desktop_ui(), use_nav());
        let env = *ui.env.read();
        let Some(OpenManage { menu, at, .. }) = self.manage.get() else { return rect().into_element() };

        let placement = match presentation(SurfaceKind::Menu, env.surface()) {
            Presentation::Anchored => MenuPlacement::Anchored { x: at.0, y: at.1 },
            _ => MenuPlacement::Centered,
        };
        let (manage, confirm, on_effect) = (self.manage, self.confirm, shell.on_effect.clone());
        let dismiss = EventHandler::new(move |()| manage.close());
        let pick = EventHandler::new(move |(level, index): (usize, usize)| {
            let Some(mut open) = manage.peek() else { return };
            let game = open.game;
            match open.menu.pick(level, index) {
                Outcome::Moved => manage.update(open),
                Outcome::Chose(action) => {
                    manage.close();
                    match action {
                        // Destructive: ask first.
                        MenuAction::Uninstall => confirm.open(game),
                        MenuAction::Properties => nav.open(Route::GameSettings { id: game }),
                        other => {
                            if let Some(effect) = other.effect(game) {
                                on_effect.call(effect);
                            }
                        }
                    }
                }
                Outcome::Closed => manage.close(),
                Outcome::None => {}
            }
        });
        ModalMenu::new(MenuLevelView::from_state(&menu), placement, env.window, env.density, pick, dismiss).into_element()
    }
}
