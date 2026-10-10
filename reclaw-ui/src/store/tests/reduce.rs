use reclaw_games::settings::{SettingKey, SettingValue};
use reclaw_runtime::RunState;

use super::state;
use crate::{
    activity::{ActivityEvent, Changelog, Kind, Stage},
    model::{AppStatus, ModProvider, ModStatus},
    nav::Route,
    notices::NoticeKind,
    settings::{SettingChange, SettingValue as UiValue, SettingsTarget},
    store::{AppAction, AppChannel},
};

#[test]
fn a_game_that_comes_to_need_an_update_raises_one_notice_and_not_again() {
    let mut s = state();
    s.games.iter_mut().for_each(|g| g.status = AppStatus::Installed);
    let mut updated = s.games.clone();
    updated[0].status = AppStatus::UpdateReady;
    assert!(s.reduce(AppAction::SetGames(updated.clone())).contains(&AppChannel::Notices));
    assert_eq!(s.notices.len(), 1);
    assert_eq!(s.notices.top().map(|n| n.kind), Some(NoticeKind::UpdateAvailable));
    s.reduce(AppAction::SetGames(updated));
    assert_eq!(s.notices.len(), 1, "the same list again says nothing new");
}

#[test]
fn run_state_changes_touch_only_the_games_and_only_when_they_change() {
    let mut s = state();
    let running = RunState::Starting;
    assert_eq!(s.reduce(AppAction::SetRun { id: 1, run: running.clone() }), vec![AppChannel::Games]);
    assert!(s.reduce(AppAction::SetRun { id: 1, run: running }).is_empty());
    assert!(s.reduce(AppAction::SetRun { id: 999, run: RunState::Idle }).is_empty());
}

#[test]
fn favorites_toggle_mirror_into_the_tags_and_are_saved() {
    let mut s = state();
    let touched = s.reduce(AppAction::ToggleFavorite(3));
    assert!(touched.contains(&AppChannel::Favorites) && touched.iter().any(|c| c.is_persisted()));
    assert!(s.games.iter().find(|g| g.id == 3).is_some_and(|g| g.is_favorite()));
    s.reduce(AppAction::ToggleFavorite(3));
    assert!(!s.games.iter().find(|g| g.id == 3).is_some_and(|g| g.is_favorite()));
    assert!(s.reduce(AppAction::ToggleFavorite(404)).is_empty(), "an unknown game is not remembered");
}

#[test]
fn saved_favorites_are_applied_to_a_library_the_host_reports_later() {
    let mut s = state();
    s.games.clear();
    s.favorites.insert(2);
    s.reduce(AppAction::SetGames(crate::fixtures::sample_games()));
    assert!(s.games.iter().find(|g| g.id == 2).is_some_and(|g| g.is_favorite()));
}

#[test]
fn a_setting_is_stored_once_and_a_repeat_changes_nothing() {
    let mut s = state();
    let change = SettingChange { app: Some(2), key: "keep_prerelease", value: UiValue::Bool(true) };
    assert_eq!(s.reduce(AppAction::Setting(change)), vec![AppChannel::Settings]);
    assert!(s.settings.toggle(SettingsTarget::App(2), "keep_prerelease", false));
    assert!(s.reduce(AppAction::Setting(change)).is_empty());
}

#[test]
fn launch_settings_are_set_and_cleared_per_game_and_for_the_defaults() {
    let mut s = state();
    let set = |app, value| AppAction::LaunchSetting { app, key: SettingKey::Vsync, value };
    assert_eq!(s.reduce(set(None, Some(SettingValue::Bool(false)))), vec![AppChannel::Launch]);
    s.reduce(set(Some(1), Some(SettingValue::Bool(true))));
    assert_eq!(s.launch.defaults.get(SettingKey::Vsync), Some(&SettingValue::Bool(false)));
    assert_eq!(s.launch.apps[&1].get(SettingKey::Vsync), Some(&SettingValue::Bool(true)));
    s.reduce(set(Some(1), None));
    assert!(!s.launch.apps.contains_key(&1));
    assert!(s.reduce(set(Some(1), None)).is_empty());
}

#[test]
fn progress_redraws_only_that_games_readers_and_a_finished_update_tells_the_user() {
    let mut s = state();
    s.activity = Default::default();
    let started =
        ActivityEvent::Started { id: 7, game_id: 2, kind: Kind::Update, title: "Skyward Quest v0.9.2".into(), bytes_total: Some(100) };
    assert_eq!(s.reduce(AppAction::Activity(started)), vec![AppChannel::ActivityOf(2)]);
    let progress = ActivityEvent::Progress { id: 7, stage: Stage::Downloading, bytes_done: 50, bytes_total: Some(100), rate: Some(10) };
    assert_eq!(s.reduce(AppAction::Activity(progress)), vec![AppChannel::ActivityOf(2)]);
    assert!(s.notices.is_empty());

    let changelog = Changelog { from: "v0.9.1".into(), to: "v0.9.2".into(), notes: "Fixes saves".into(), url: None };
    let touched = s.reduce(AppAction::Activity(ActivityEvent::Finished { id: 7, changelog: Some(changelog) }));
    assert_eq!(touched, vec![AppChannel::ActivityOf(2), AppChannel::Notices]);
    let notice = s.notices.top().expect("a notice");
    assert_eq!(
        (notice.kind, notice.title.as_str(), notice.details.clone()),
        (NoticeKind::UpdateFinished, "Skyward Quest updated", vec!["Fixes saves".to_string()])
    );
}

#[test]
fn a_failed_job_and_a_late_event_for_a_job_that_is_gone() {
    let mut s = state();
    s.activity = Default::default();
    s.reduce(AppAction::Activity(ActivityEvent::Started {
        id: 1,
        game_id: 5,
        kind: Kind::Install,
        title: "Moon Garden".into(),
        bytes_total: None,
    }));
    s.reduce(AppAction::Activity(ActivityEvent::Failed { id: 1, reason: "no space left".into(), details: Vec::new() }));
    assert_eq!(s.notices.top().map(|n| (n.kind, n.body.as_str())), Some((NoticeKind::DownloadFailed, "no space left")));
    assert!(
        s.reduce(AppAction::Activity(ActivityEvent::Progress {
            id: 1,
            stage: Stage::Downloading,
            bytes_done: 1,
            bytes_total: None,
            rate: None
        }))
        .is_empty()
    );
}

#[test]
fn a_mod_that_finishes_installing_raises_a_notice() {
    let mut s = state();
    let id = "ghost-data".to_string();
    assert_eq!(s.mods.iter().find(|m| m.id == id).map(|m| m.status), Some(ModStatus::Installing));
    s.reduce(AppAction::SetModStatus { provider: ModProvider::Thunderstore, id, status: ModStatus::Installed });
    assert_eq!(s.notices.top().map(|n| n.kind), Some(NoticeKind::ModInstalled));
}

#[test]
fn notices_are_dismissed_one_at_a_time_or_all_together() {
    let mut s = state();
    s.reduce(AppAction::SetStatus { id: 1, status: AppStatus::UpdateReady });
    s.reduce(AppAction::SetStatus { id: 6, status: AppStatus::Failed });
    s.reduce(AppAction::SetStatus { id: 6, status: AppStatus::UpdateReady });
    assert_eq!(s.notices.len(), 2);
    let top = s.notices.top().map(|n| n.id).expect("notice");
    assert_eq!(s.reduce(AppAction::DismissNotice(top)), vec![AppChannel::Notices]);
    assert_eq!(s.reduce(AppAction::DismissAllNotices), vec![AppChannel::Notices]);
    assert!(s.reduce(AppAction::DismissAllNotices).is_empty());
}

#[test]
fn the_mailbox_holds_a_request_until_the_ui_takes_it() {
    let mut s = state();
    assert_eq!(s.reduce(AppAction::Open(Some(Route::Mods {}))), vec![AppChannel::Mailbox]);
    assert_eq!(s.mailbox.open, Some(Route::Mods {}));
    s.reduce(AppAction::Open(None));
    assert_eq!(s.mailbox.open, None);
}

#[test]
fn unchanged_device_facts_touch_nothing() {
    let mut s = state();
    assert!(s.reduce(AppAction::SetKeyboardInset(0.)).is_empty());
    assert_eq!(s.reduce(AppAction::SetKeyboardInset(216.)), vec![AppChannel::Keyboard]);
    assert!(s.reduce(AppAction::SetController(None)).is_empty());
}

#[test]
fn an_activity_channel_reaches_the_aggregate_readers_too() {
    use freya::radio::RadioChannel;
    assert_eq!(AppChannel::ActivityOf(4).derive_channel(&state()), vec![AppChannel::ActivityOf(4), AppChannel::Activity]);
    assert_eq!(AppChannel::Games.derive_channel(&state()), vec![AppChannel::Games]);
}

#[test]
fn the_catalog_status_is_set_and_redraws_only_its_readers_when_it_changes() {
    use crate::store::{AppState, CatalogPhase, CatalogStatus};
    let mut s = AppState::default();
    let loading = CatalogStatus { phase: CatalogPhase::Loading, ..Default::default() };
    assert_eq!(s.reduce(AppAction::Catalog(loading.clone())), vec![AppChannel::Catalog]);
    assert_eq!(s.catalog, loading);
    assert!(s.reduce(AppAction::Catalog(loading)).is_empty(), "the same status again tells nobody");
}

#[test]
fn a_notice_from_the_host_is_queued_and_shown() {
    use crate::{
        notices::{Notice, NoticeKind},
        store::AppState,
    };
    let mut s = AppState::default();
    let touched = s.reduce(AppAction::Notify(Notice::problem("Catalog", "offline", vec!["no network".into()])));
    assert_eq!(touched, vec![AppChannel::Notices]);
    let top = s.notices.top().expect("queued");
    assert_eq!((top.kind, top.title.as_str()), (NoticeKind::Problem, "Catalog"));
    assert!(top.kind.is_failure(), "a problem stays until dismissed");
    assert!(!Notice::note("t", "b", vec![]).kind.is_failure(), "a note passes");
}

#[test]
fn rebuilding_the_game_list_keeps_what_is_running() {
    let mut s = state();
    let id = s.games[0].id;
    s.reduce(AppAction::SetRun { id, run: RunState::Running { pid: 7, since: std::time::SystemTime::now() } });
    let mut rebuilt = s.games.clone();
    for game in &mut rebuilt {
        game.run = RunState::Idle;
        game.version = "v9".into();
    }
    s.reduce(AppAction::SetGames(rebuilt));
    assert!(matches!(s.games[0].run, RunState::Running { pid: 7, .. }), "the host's rebuild does not know what runs");
    assert_eq!(s.games[0].version, "v9", "the rest of the new list is taken");
}

#[test]
fn the_moddable_games_are_told_on_the_mods_channel_only_when_they_change() {
    let mut s = state();
    assert_eq!(s.reduce(AppAction::SetModdable(vec![2, 1])), vec![AppChannel::Mods]);
    assert_eq!(s.moddable, vec![2, 1]);
    assert!(s.reduce(AppAction::SetModdable(vec![2, 1])).is_empty());
}
