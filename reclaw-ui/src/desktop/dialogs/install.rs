use freya::prelude::*;

use super::Slot;
use crate::{
    components::InstallDialog, desktop::use_desktop_ui, effect::Effect, settings::default_install_location, shell::use_shell,
    store::use_games,
};

/// The install form, over whichever page opened it. Owns the form's fields.
#[derive(Clone, PartialEq)]
pub(super) struct InstallSheet {
    pub open: Slot<u32>,
}

impl Component for InstallSheet {
    fn render(&self) -> impl IntoElement {
        let (shell, ui) = (use_shell(), use_desktop_ui());
        let env = *ui.env.read();
        let open = self.open;
        let location = use_state(String::new);
        let prerelease = use_state(|| false);
        let (store, games) = (shell.store, use_games());

        // Each opening starts from the Library default as it is now. Read from the store, not from a
        // value captured at render: this effect is created once.
        use_side_effect(move || {
            if open.get().is_some() {
                let mut location = location;
                location.set(store.with(|s| default_install_location(&s.settings)));
            }
        });

        let id = open.get();
        let game = id.and_then(|id| games.iter().find(|g| g.id == id));
        let title = game.map(|g| g.title.to_string()).unwrap_or_default();

        let close_effect = shell.on_effect.clone();
        let close = move || open.close();
        let (close_for_cancel, close_for_confirm) = (close, close);

        InstallDialog::new(game.is_some(), &title, location, prerelease)
            .surface(env.surface())
            .window(env.window)
            .keyboard_inset(env.keyboard_inset)
            .on_cancel(move |()| close_for_cancel())
            .on_confirm(move |()| {
                if let Some(app) = id {
                    close_effect.call(Effect::StartInstall { app, location: location.read().clone(), prerelease: *prerelease.read() });
                }
                close_for_confirm();
            })
    }
}
