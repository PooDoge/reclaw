//! What pressing a focus target does.
use reclaw_input::FocusId;

use super::{ConfirmKind, DeckState, DeckView, Effect, MAIN_MENU, MainMenuEntry, Overlay, Screen, Section, ids};
use crate::{
    deck::{LaunchVerb, launch_verb},
    model::GameEntry,
};

impl DeckState {
    pub(super) fn activate(&mut self, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        if id == ids::PAGE_BACK {
            return self.back(view, fx);
        }
        // Confirmation buttons first: they are modal.
        if let Overlay::Confirm(kind) = self.overlay {
            return self.activate_confirm(kind, id, view, fx);
        }
        if let Overlay::Notice(notice) = self.overlay {
            return self.activate_notice(notice, id, view, fx);
        }
        if let Some(app) = view.active_game().map(|g| g.id) {
            if id == ids::BANNER_RESUME || id == ids::QA_RESUME {
                return self.resume(app, view, fx);
            }
            if id == ids::BANNER_STOP || id == ids::QA_STOP {
                return fx.push(Effect::Stop(app));
            }
        }
        if let Some(game) = ids::tile_game(id) {
            return self.go(Screen::Game(game), view);
        }
        if let Some(index) = ids::menu_index(id) {
            return self.activate_main_menu(MAIN_MENU.get(index).copied(), view, fx);
        }
        if let Some(job) = ids::cancel_activity(id) {
            // A running job is cancelled by the host; an ended row is just removed from the list.
            let running = view.downloads.iter().find(|a| a.id == job).is_some_and(|a| a.is_running());
            return fx.push(if running { Effect::CancelActivity(job) } else { Effect::DismissActivity(job) });
        }
        if id == ids::QA_DOWNLOADS {
            return self.go_to_section(Section::Downloads, view);
        }
        if let Some(index) = ids::qa_recent_index(id) {
            // Jump to a page visited lately. The router moves and this follows it; the panel closes first.
            if let Some(route) = view.recents.get(index) {
                self.came_from_game = false;
                self.close_overlay_quietly(view);
                fx.push(Effect::Navigate(route.clone()));
            }
            return;
        }
        match self.screen {
            Screen::Game(game_id) => {
                if let Some(game) = view.game(game_id) {
                    self.activate_game(game, id, view, fx);
                }
            }
            Screen::Install(app) => self.activate_install(app, id, view, fx),
            Screen::Settings(target) => self.activate_settings(target, id, view, fx),
            Screen::Home => {}
        }
    }

    fn activate_game(&mut self, game: &GameEntry, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        match id {
            i if i == ids::GAME_PRIMARY => self.primary(game, view, fx),
            i if i == ids::GAME_STOP => fx.push(Effect::Stop(game.id)),
            i if i == ids::GAME_FOLDER => fx.push(Effect::OpenFolder(game.id)),
            i if i == ids::GAME_MANAGE => self.open_options(view),
            _ => {}
        }
    }

    fn primary(&mut self, game: &GameEntry, view: &DeckView, fx: &mut Vec<Effect>) {
        match launch_verb(game.status, &game.run, true) {
            LaunchVerb::Play => self.launch(game.id, fx),
            LaunchVerb::Retry if game.status.is_installed() => self.launch(game.id, fx),
            LaunchVerb::Retry | LaunchVerb::Install => self.open_install(game.id, view),
            LaunchVerb::Update => fx.push(Effect::Update(game.id)),
            LaunchVerb::Resume | LaunchVerb::Stop => self.resume(game.id, view, fx),
            LaunchVerb::ForceQuit => fx.push(Effect::Stop(game.id)),
            LaunchVerb::Installing | LaunchVerb::Starting => {}
        }
    }

    fn launch(&mut self, app: u32, fx: &mut Vec<Effect>) {
        self.in_front = false;
        fx.push(Effect::Launch(app));
    }

    fn activate_main_menu(&mut self, entry: Option<MainMenuEntry>, view: &DeckView, fx: &mut Vec<Effect>) {
        match entry {
            Some(MainMenuEntry::Section(s)) => self.go_to_section(s, view),
            Some(MainMenuEntry::Settings) => {
                self.close_overlay_quietly(view);
                self.open_settings(super::SettingsTarget::Global, view);
            }
            Some(MainMenuEntry::SwitchToDesktop) => fx.push(Effect::SwitchToDesktop),
            None => {}
        }
    }

    fn activate_notice(&mut self, notice: crate::notices::NoticeId, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        match id {
            i if i == ids::NOTICE_CLOSE => self.close_overlay_quietly(view),
            i if i == ids::NOTICE_DISMISS => {
                self.close_overlay_quietly(view);
                fx.push(Effect::DismissNotice(notice));
            }
            _ => {}
        }
    }

    fn activate_confirm(&mut self, kind: ConfirmKind, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        match (kind, id) {
            (_, i) if i == ids::CONFIRM_CANCEL => self.close_overlay_quietly(view),
            (ConfirmKind::Uninstall(app), i) if i == ids::CONFIRM_OK => {
                self.close_overlay_quietly(view);
                // The app is going away; its pages go with it.
                self.stack.clear();
                self.screen = Screen::Home;
                self.enter_scope(view);
                self.request_route();
                fx.push(Effect::Uninstall(app));
            }
            _ => {}
        }
    }
}
