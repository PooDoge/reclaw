use super::{action::AppAction, changes, channel::AppChannel, state::AppState};
use crate::{
    activity::{Activity, ActivityEvent, Change, Kind, Outcome},
    model::{AppStatus, ModStatus},
    notices::Notice,
    settings::{SettingChange, SettingsTarget},
};

/// The channels a change touched; empty when nothing changed, so no reader redraws and nothing is saved.
type Touched = Vec<AppChannel>;

impl AppState {
    /// Apply one action. Pure: no handles, no time, no I/O. Returns what changed.
    pub fn reduce(&mut self, action: AppAction) -> Touched {
        match action {
            AppAction::SetGames(mut games) => {
                // The host rebuilds the list from the library and what is on disk; what is running is not in either.
                for game in &mut games {
                    if let Some(old) = self.games.iter().find(|old| old.id == game.id) {
                        game.run = old.run.clone();
                    }
                }
                self.notice_new_updates(&games);
                changes::sync_favorite_tags(&mut games, &self.favorites);
                self.games = games;
                vec![AppChannel::Games, AppChannel::Notices]
            }
            AppAction::SetProjects(projects) => set_if_changed(&mut self.projects, projects, AppChannel::Projects),
            AppAction::SetMods(mods) => set_if_changed(&mut self.mods, mods, AppChannel::Mods),
            AppAction::Catalog(status) => set_if_changed(&mut self.catalog, status, AppChannel::Catalog),
            AppAction::Credentials { provider, status } => {
                if self.credentials.of(provider) == &status {
                    return Vec::new();
                }
                self.credentials.set(provider, status);
                vec![AppChannel::Credentials]
            }
            AppAction::Notify(notice) => {
                self.notices.push(notice);
                vec![AppChannel::Notices]
            }
            AppAction::SetRun { id, run } => match self.games.iter_mut().find(|g| g.id == id) {
                Some(game) if game.run != run => {
                    game.run = run;
                    vec![AppChannel::Games]
                }
                _ => Vec::new(),
            },
            AppAction::SetStatus { id, status } => self.set_status(id, status),
            AppAction::SetModStatus { provider, id, status } => self.set_mod_status(provider, &id, status),
            AppAction::Activity(event) => self.apply_activity(event),
            AppAction::SetController(controller) => set_if_changed(&mut self.controller, controller, AppChannel::Controller),
            AppAction::SetKeyboardInset(px) => set_if_changed(&mut self.keyboard_inset, px, AppChannel::Keyboard),
            AppAction::SetDisplay(display) => set_if_changed(&mut self.display, display, AppChannel::Display),
            AppAction::Open(route) => set_if_changed(&mut self.mailbox.open, route, AppChannel::Mailbox),

            AppAction::ToggleFavorite(id) => {
                if !self.games.iter().any(|g| g.id == id) && !self.favorites.contains(&id) {
                    return Vec::new();
                }
                if !self.favorites.remove(&id) {
                    self.favorites.insert(id);
                }
                changes::sync_favorite_tags(&mut self.games, &self.favorites);
                vec![AppChannel::Games, AppChannel::Favorites]
            }
            AppAction::Setting(SettingChange { app, key, value }) => {
                let target = app.map_or(SettingsTarget::Global, SettingsTarget::App);
                if self.settings.get(target, key) == Some(value) {
                    return Vec::new();
                }
                self.settings.set(target, key, value);
                vec![AppChannel::Settings]
            }
            AppAction::SettingText { app, field, value } => {
                let (Some(key), target) = (field.key(), app.map_or(SettingsTarget::Global, SettingsTarget::App)) else { return Vec::new() };
                if self.settings.text(target, key) == Some(value.as_str()) {
                    return Vec::new();
                }
                self.settings.set_text(target, key, value);
                vec![AppChannel::Settings]
            }
            AppAction::LaunchSetting { app, key, value } => {
                let before = self.launch.clone();
                changes::set_launch_setting(&mut self.launch.defaults, &mut self.launch.apps, app, key, value);
                if self.launch == before { Vec::new() } else { vec![AppChannel::Launch] }
            }
            AppAction::MarkModInstalling { provider, id } => {
                if changes::mark_mod_installing(&mut self.mods, provider, &id) {
                    vec![AppChannel::Mods]
                } else {
                    Vec::new()
                }
            }
            AppAction::DismissNotice(id) => {
                if self.notices.dismiss(id) {
                    vec![AppChannel::Notices]
                } else {
                    Vec::new()
                }
            }
            AppAction::DismissAllNotices => {
                if self.notices.dismiss_all() > 0 {
                    vec![AppChannel::Notices]
                } else {
                    Vec::new()
                }
            }
            AppAction::Window(window) => set_if_changed(&mut self.window, window, AppChannel::Window),
        }
    }

    fn game_title(&self, id: u32) -> Option<String> {
        self.games
            .iter()
            .find(|g| g.id == id)
            .map(|g| g.title.to_string())
            .or_else(|| self.projects.iter().find(|p| p.id == id).map(|p| p.title.clone()))
    }

    /// A game that has just come to need an update gets a notice. Games already waiting are not repeated.
    fn notice_new_updates(&mut self, incoming: &[crate::model::GameEntry]) {
        for game in incoming.iter().filter(|g| g.status == AppStatus::UpdateReady) {
            let was_waiting = self.games.iter().any(|old| old.id == game.id && old.status == AppStatus::UpdateReady);
            if !was_waiting {
                self.notices.push(Notice::update_available(game.id, &game.title));
            }
        }
    }

    fn set_status(&mut self, id: u32, status: AppStatus) -> Touched {
        let Some(game) = self.games.iter_mut().find(|g| g.id == id) else { return Vec::new() };
        if game.status == status {
            return Vec::new();
        }
        game.status = status;
        let title = game.title.to_string();
        if status == AppStatus::UpdateReady {
            self.notices.push(Notice::update_available(id, &title));
        }
        vec![AppChannel::Games, AppChannel::Notices]
    }

    fn set_mod_status(&mut self, provider: crate::model::ModProvider, id: &str, status: ModStatus) -> Touched {
        let Some(entry) = self.mods.iter_mut().find(|m| m.provider == provider && m.id == id) else { return Vec::new() };
        if entry.status == status {
            return Vec::new();
        }
        let finished = entry.status == ModStatus::Installing && status == ModStatus::Installed;
        entry.status = status;
        let (game, title) = (entry.game_id, entry.title.clone());
        if finished {
            self.notices.push(Notice::mod_installed(game, &title));
        }
        vec![AppChannel::Mods, AppChannel::Notices]
    }

    /// Run the event on the board, then tell the user about how a job ended.
    fn apply_activity(&mut self, event: ActivityEvent) -> Touched {
        let id = match &event {
            ActivityEvent::Started { id, .. }
            | ActivityEvent::Progress { id, .. }
            | ActivityEvent::Finished { id, .. }
            | ActivityEvent::Failed { id, .. }
            | ActivityEvent::Log { id, .. }
            | ActivityEvent::Cancelled { id }
            | ActivityEvent::Dismiss { id } => *id,
        };
        let Some((game, change)) = self.activity.apply(event) else { return Vec::new() };
        let mut touched = vec![AppChannel::ActivityOf(game)];
        if let (true, Some(activity)) = (matches!(change, Change::Finished | Change::Failed), self.activity.get(id).cloned()) {
            let game_title = self.game_title(game).unwrap_or_else(|| activity.title.clone());
            self.notices.push(notice_for(&activity, &game_title));
            touched.push(AppChannel::Notices);
        }
        touched
    }
}

/// What to tell the user when a job ends.
fn notice_for(activity: &Activity, game_title: &str) -> Notice {
    match (&activity.outcome, &activity.kind) {
        (Outcome::Failed { reason }, _) => Notice::download_failed(activity.game_id, &activity.title, reason, &activity.details),
        (_, Kind::Update) => Notice::update_finished(activity.game_id, game_title, activity.changelog.as_ref()),
        (_, Kind::Install) => Notice::install_finished(activity.game_id, game_title),
        (_, Kind::Mod { .. }) => Notice::mod_installed(activity.game_id, &activity.title),
    }
}

fn set_if_changed<T: PartialEq>(field: &mut T, value: T, channel: AppChannel) -> Touched {
    if *field == value {
        Vec::new()
    } else {
        *field = value;
        vec![channel]
    }
}
