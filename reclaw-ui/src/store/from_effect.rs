use super::action::AppAction;
use crate::{activity::ActivityEvent, effect::Effect};

/// The effects that only change state the UI owns. The shell applies these to the store itself and
/// still passes the effect on, so a host can mirror them (to sync, to log) but does not have to.
pub fn ui_action(effect: &Effect) -> Option<AppAction> {
    Some(match effect {
        Effect::ToggleFavorite(id) => AppAction::ToggleFavorite(*id),
        Effect::Setting(change) => AppAction::Setting(*change),
        Effect::TextCommitted { app, field, value } => AppAction::SettingText { app: *app, field: *field, value: value.clone() },
        Effect::LaunchSetting { app, key, value } => AppAction::LaunchSetting { app: *app, key: *key, value: value.clone() },
        Effect::InstallMod { provider, id } => AppAction::MarkModInstalling { provider: *provider, id: id.clone() },
        Effect::DismissActivity(id) => AppAction::Activity(ActivityEvent::Dismiss { id: *id }),
        Effect::DismissNotice(id) => AppAction::DismissNotice(*id),
        Effect::DismissAllNotices => AppAction::DismissAllNotices,
        _ => return None,
    })
}
