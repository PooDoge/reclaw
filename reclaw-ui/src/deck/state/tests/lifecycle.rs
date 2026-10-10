//! Play, Guide, Resume, Stop: who owns the pad and what the host is told.
use super::support::*;

#[test]
fn play_launches_and_hands_the_pad_to_the_app() {
    let mut f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Confirm]); // open Starfall 64 (installed)
    assert_eq!(s.apply(Confirm, &f.view()), vec![Effect::Launch(1)]);
    assert!(!s.in_front());
    f.set_run(1, running());
    assert_eq!(s.sync(&f.view()), vec![Effect::InputOwner(InputOwner::App)]);
    assert_eq!(s.input_owner(&f.view()), InputOwner::App);
}

#[test]
fn guide_over_a_running_app_brings_us_forward_and_back_closes_it_again() {
    let mut f = Fixture::new();
    let mut s = f.with_app_running();
    assert_eq!(s.apply(MainMenu, &f.view()), vec![Effect::BringLauncherToFront, Effect::InputOwner(InputOwner::Launcher)]);
    assert_eq!(s.overlay(), Overlay::MainMenu);
    assert_eq!(s.apply(Back, &f.view()), vec![Effect::SendLauncherToBack, Effect::InputOwner(InputOwner::App)]);
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn resume_from_quick_access_returns_to_the_game() {
    let mut f = Fixture::new();
    let mut s = f.with_app_running();
    press(&mut s, &f, &[QuickAccess]);
    assert_eq!(s.focus(), ids::QA_RESUME, "Resume is the default focus while an app runs");
    assert_eq!(s.apply(Confirm, &f.view()), vec![Effect::Resume(1), Effect::InputOwner(InputOwner::App)]);
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn stop_and_force_stop_are_the_same_effect() {
    let mut f = Fixture::new();
    f.set_run(1, running());
    let mut s = f.state();
    press(&mut s, &f, &[go(Right)]);
    assert_eq!(s.focus(), ids::BANNER_STOP);
    assert_eq!(s.apply(Confirm, &f.view()), vec![Effect::Stop(1)]);
    f.set_run(1, RunState::Stopping { pid: 7 });
    assert_eq!(s.apply(Confirm, &f.view()), vec![Effect::Stop(1)], "pressed again: the supervisor force-kills");
}

#[test]
fn game_page_shows_resume_and_stop_for_a_running_app() {
    let mut f = Fixture::new();
    f.set_run(1, running());
    let mut s = f.state();
    press(&mut s, &f, &[go(Down), Confirm]);
    assert_eq!(s.screen(), Screen::Game(1));
    press(&mut s, &f, &[go(Right)]);
    assert_eq!(s.focus(), ids::GAME_STOP);
    press(&mut s, &f, &[go(Left)]);
    assert_eq!(s.apply(Confirm, &f.view()), vec![Effect::Resume(1), Effect::InputOwner(InputOwner::App)]);
}

#[test]
fn app_ending_brings_the_launcher_back() {
    let mut f = Fixture::new();
    let mut s = f.with_app_running();
    f.set_run(1, RunState::Failed(reclaw_runtime::Outcome::ExitedWithCode { code: 2, ran_for: Default::default() }));
    assert_eq!(s.on_app_ended(&f.view()), vec![Effect::BringLauncherToFront, Effect::InputOwner(InputOwner::Launcher)]);
}

#[test]
fn update_does_not_need_the_install_page() {
    let f = Fixture::new();
    let mut s = f.state();
    s.click(tile(0, 2), &f.view()); // Skyward Quest: update ready, so on Continue
    assert_eq!(s.apply(Confirm, &f.view()), vec![Effect::Update(2)]);
    assert!(matches!(s.screen(), Screen::Game(2)));
}
