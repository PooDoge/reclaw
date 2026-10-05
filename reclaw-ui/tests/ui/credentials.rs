//! Access tokens and diagnostics on both interfaces: what the Network section shows, that a pasted token leaves the page exactly
//! once and as a `Secret`, and that the buttons ask the host for what their labels say.
use freya::prelude::NamedKey::*;
use reclaw_games::settings::{DisplayEnvironment, all_specs};
use reclaw_log::Secret;
use reclaw_net::{Provider, Quota};
use reclaw_ui::{
    credentials::{TokenCheck, TokenSource, TokenStatus},
    effect::Effect,
    nav::Route,
    settings::{SettingChange, SettingValue, global_settings},
    shell::{DevOverrides, MotionOverride},
    store::AppAction,
};

use crate::common::*;

const TOKEN: &str = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";

fn calm() -> DevOverrides {
    DevOverrides { motion: Some(MotionOverride::Reduced), ..DevOverrides::default() }
}

/// Tall enough to show both services' groups at once, so every row can be clicked where it is.
fn network() -> Session {
    let mut s = Mount::desktop().size(1280., 1900.).dev(calm()).start_at(Route::Settings {});
    s.click_label("Network");
    s
}

/// Both services have a "Save token", a "Check token" and a "Remove the saved token": click the one nearest the top, GitHub's.
fn click_first(s: &mut Session, text: &str) {
    let (left, top, right, bottom) = s.label_box(text).unwrap_or_else(|| panic!("no label {text:?} in {:?}", s.labels()));
    s.runner.click_cursor((f64::from((left + right) / 2.), f64::from((top + bottom) / 2.)));
    s.settle();
}

fn has_ignoring_case(s: &Session, text: &str) -> bool {
    s.labels().iter().any(|l| l.eq_ignore_ascii_case(text))
}

/// Click into the first token box: it sits under the row's description.
fn click_token_box(s: &mut Session) {
    let (left, _, _, bottom) = s.label_box("Paste it here, then choose Save token. It is not shown again.").expect("the description");
    s.runner.click_cursor((f64::from(left + 80.), f64::from(bottom + 24.)));
    s.settle();
}

#[test]
fn the_network_section_has_a_group_per_service_with_the_rows_in_the_order_people_use_them() {
    let s = network();
    for heading in ["GitHub", "GitLab"] {
        assert!(has_ignoring_case(&s, heading), "{heading}: {:?}", s.labels());
    }
    for row in
        ["Status", "Token", "Save token", "Check token", "Create a token on GitHub", "Create a token on GitLab", "Remove the saved token"]
    {
        assert!(s.has_label(row), "{row}: {:?}", s.labels());
    }
    assert!(s.has_label("No token (60 requests an hour)") && s.has_label("No token"), "{:?}", s.labels());
    let (_, github, _, _) = s.label_box("Create a token on GitHub").expect("github");
    let (_, gitlab, _, _) = s.label_box("Create a token on GitLab").expect("gitlab");
    assert!(github < gitlab, "GitHub's group is first");
}

#[test]
fn the_network_section_looks_right() {
    let mut s = Mount::desktop().size(1280., 1000.).dev(calm()).start_at(Route::Settings {});
    s.click_label("Network");
    s.snapshot("desktop-settings-network");
    let mut deck = deck_to_network_rows();
    deck.snapshot("deck-settings-network");
}

#[test]
fn the_status_row_follows_what_the_host_reports() {
    let mut s = network();
    let accepted = TokenCheck::Accepted {
        had_token: true,
        quota: Some(Quota { limit: 5000, remaining: 4987, reset_at: 0 }),
        expires: None,
        extra_permissions: vec![],
    };
    s.dispatch(AppAction::Credentials { provider: Provider::GitHub, status: TokenStatus { source: TokenSource::Saved, check: accepted } });
    s.settle();
    assert!(s.has_label("4,987 of 5,000 left"), "{:?}", s.labels());
    s.dispatch(AppAction::Credentials {
        provider: Provider::GitLab,
        status: TokenStatus { source: TokenSource::Saved, check: TokenCheck::Rejected },
    });
    s.settle();
    assert!(s.has_label("GitLab refused it"), "{:?}", s.labels());
}

#[test]
fn a_pasted_token_is_sent_once_as_a_secret_and_the_box_is_emptied() {
    let mut s = network();
    click_token_box(&mut s);
    s.type_text(TOKEN);
    assert!(!s.labels().iter().any(|l| l.contains(TOKEN)), "the box shows dots, not the token: {:?}", s.labels());
    assert!(s.take_effects().is_empty(), "typing sends nothing: only Save token does");

    click_first(&mut s, "Save token");
    let effects = s.take_effects();
    assert_eq!(effects, vec![Effect::SaveToken { provider: Provider::GitHub, token: Secret::new(TOKEN) }]);
    assert!(!format!("{effects:?}").contains("0123456789"), "an effect prints as nothing: {effects:?}");

    click_first(&mut s, "Save token");
    assert_eq!(
        s.take_effects(),
        vec![Effect::SaveToken { provider: Provider::GitHub, token: Secret::new("") }],
        "the box was emptied, so a second press sends nothing to keep (the host says so)"
    );
}

#[test]
fn the_other_buttons_ask_the_host_for_what_they_say() {
    let mut s = network();
    click_first(&mut s, "Check token");
    click_first(&mut s, "Create a token on GitHub");
    click_first(&mut s, "Create a token on GitLab");
    click_first(&mut s, "Remove the saved token");
    assert_eq!(
        s.take_effects(),
        vec![
            Effect::CheckToken(Provider::GitHub),
            Effect::OpenUrl(Provider::GitHub.token_page().to_string()),
            Effect::OpenUrl(Provider::GitLab.token_page().to_string()),
            Effect::RemoveToken(Provider::GitHub),
        ]
    );
}

#[test]
fn diagnostics_offers_the_log_level_the_folder_and_the_report() {
    let mut s = Mount::desktop().dev(calm()).start_at(Route::Settings {});
    s.click_label("Diagnostics");
    for row in ["Log detail", "Open the log folder", "Save a diagnostics report"] {
        assert!(s.has_label(row), "{row}: {:?}", s.labels());
    }
    s.click_label("Open the log folder");
    s.click_label("Save a diagnostics report");
    assert_eq!(s.take_effects(), vec![Effect::OpenLogFolder, Effect::SaveDiagnostics]);

    s.click_label("Log detail");
    assert!(s.has_label("Problems only") && s.has_label("Detailed"), "the picker lists the levels: {:?}", s.labels());
    s.click_label("Detailed");
    assert!(s.take_effects().contains(&Effect::Setting(SettingChange { app: None, key: "log_level", value: SettingValue::Choice(2) })));
}

fn deck_to_network_rows() -> Session {
    let mut s = Mount::deck().start();
    s.press(Tab);
    s.presses(&[ArrowDown, ArrowDown, ArrowDown, ArrowDown, Enter]);
    let env = DisplayEnvironment::unknown();
    let schema = global_settings(&all_specs(&env), &env);
    let at = schema.sections.iter().position(|section| section.id == "network").expect("the Network section");
    for _ in 0..at {
        s.press(ArrowDown);
    }
    s.press(Enter);
    s
}

#[test]
fn on_deck_the_token_is_typed_into_its_box_and_saved_by_its_own_row_without_ever_being_a_setting() {
    let mut s = deck_to_network_rows();
    assert!(s.has_label("Save token") && s.has_label("Check token"), "{:?}", s.labels());
    s.press(ArrowDown); // Status -> Token
    s.press(Enter);
    assert!(s.effects().contains(&Effect::BeginTextEntry(reclaw_ui::settings::TextField::GithubToken)), "{:?}", s.effects());
    s.type_text(TOKEN);
    s.press(Enter); // done typing
    let after_typing = s.take_effects();
    assert!(
        !after_typing.iter().any(|e| matches!(e, Effect::TextCommitted { .. })),
        "a token is never reported as a setting's text: {after_typing:?}"
    );
    assert!(after_typing.contains(&Effect::EndTextEntry(reclaw_ui::settings::TextField::GithubToken)));

    s.press(ArrowDown); // Token -> Save token
    s.press(Enter);
    let sent = s.take_effects();
    assert_eq!(sent, vec![Effect::SaveToken { provider: Provider::GitHub, token: Secret::new(TOKEN) }], "{sent:?}");
    assert!(!format!("{sent:?}").contains("0123456789"));

    s.press(Enter);
    assert_eq!(
        s.take_effects(),
        vec![Effect::SaveToken { provider: Provider::GitHub, token: Secret::new("") }],
        "handed over means emptied"
    );
}

#[test]
fn on_deck_check_create_and_remove_are_rows_too() {
    let mut s = deck_to_network_rows();
    s.presses(&[ArrowDown, ArrowDown, ArrowDown]); // Status, Token, Save -> Check
    s.press(Enter);
    s.press(ArrowDown); // Create
    s.press(Enter);
    s.press(ArrowDown); // Remove
    s.press(Enter);
    assert_eq!(
        s.take_effects(),
        vec![
            Effect::CheckToken(Provider::GitHub),
            Effect::OpenUrl(Provider::GitHub.token_page().to_string()),
            Effect::RemoveToken(Provider::GitHub)
        ]
    );
}
