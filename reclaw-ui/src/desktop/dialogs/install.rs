use freya::prelude::*;

use super::Slot;
use crate::{components::InstallDialog, desktop::use_desktop_ui, effect::Effect, shell::use_shell};

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
        let location = use_state(|| String::from("~/Reclaw/Apps"));
        let mut game_file = use_state(|| None::<String>);
        let shortcut = use_state(|| true);
        let prerelease = use_state(|| false);
        let mut chosen = shell.host.chosen_file;

        // The host answers `ChooseFile` by writing the path here; take it for the open form.
        use_side_effect(move || {
            let picked = chosen.read().clone();
            if let (Some(path), Some(_)) = (picked, open.peek()) {
                chosen.set(None);
                game_file.set(Some(path));
            }
        });

        let id = open.get();
        let games = shell.host.games.read();
        let game = id.and_then(|id| games.iter().find(|g| g.id == id));
        let title = game.map(|g| g.title.to_string()).unwrap_or_default();

        let (on_effect, close_effect) = (shell.on_effect.clone(), shell.on_effect.clone());
        let close = move || {
            let mut game_file = game_file;
            game_file.set(None);
            open.close();
        };
        let (close_for_cancel, close_for_confirm) = (close, close);

        InstallDialog::new(game.is_some(), &title, location, game_file, shortcut, prerelease)
            .surface(env.surface())
            .window(env.window)
            .keyboard_inset(env.keyboard_inset)
            .on_choose_file(move |()| {
                if let Some(app) = id {
                    on_effect.call(Effect::ChooseFile(app));
                }
            })
            .on_cancel(move |()| close_for_cancel())
            .on_confirm(move |()| {
                if let Some(app) = id {
                    close_effect.call(Effect::StartInstall {
                        app,
                        location: location.read().clone(),
                        game_file: game_file.read().clone(),
                        shortcut: *shortcut.read(),
                        prerelease: *prerelease.read(),
                    });
                }
                close_for_confirm();
            })
    }
}
