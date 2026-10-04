//! Tests for the mapper: sticks, triggers, the held-button rules and the Guide button.
use super::*;

fn mapper() -> (InputMapper, Instant) {
    (InputMapper::new(ActionMap::default(), MapperConfig::default()), Instant::now())
}

fn press(button: Button) -> RawEvent {
    RawEvent::Button { button, pressed: true }
}

fn release(button: Button) -> RawEvent {
    RawEvent::Button { button, pressed: false }
}

#[test]
fn face_buttons_map_on_press_only() {
    let (mut m, t) = mapper();
    assert_eq!(m.handle(press(Button::South), t), vec![Action::Confirm]);
    assert!(m.handle(release(Button::South), t).is_empty());
}

#[test]
fn dpad_moves_then_repeats_after_the_delay() {
    let (mut m, t) = mapper();
    assert_eq!(m.handle(press(Button::DPadRight), t), vec![Action::Navigate(Direction::Right)]);
    assert!(m.tick(t + Duration::from_millis(399)).is_empty());
    assert_eq!(m.tick(t + Duration::from_millis(400)), vec![Action::Navigate(Direction::Right)]);
    assert!(m.tick(t + Duration::from_millis(450)).is_empty());
    assert_eq!(m.tick(t + Duration::from_millis(510)), vec![Action::Navigate(Direction::Right)]);
}

#[test]
fn releasing_stops_the_repeat() {
    let (mut m, t) = mapper();
    m.handle(press(Button::DPadDown), t);
    m.handle(release(Button::DPadDown), t + Duration::from_millis(100));
    assert!(m.tick(t + Duration::from_secs(2)).is_empty());
}

#[test]
fn stall_does_not_burst() {
    let (mut m, t) = mapper();
    m.handle(press(Button::DPadUp), t);
    assert_eq!(m.tick(t + Duration::from_secs(5)).len(), 1);
}

#[test]
fn newest_dpad_direction_wins_and_falls_back() {
    let (mut m, t) = mapper();
    m.handle(press(Button::DPadUp), t);
    assert_eq!(m.handle(press(Button::DPadLeft), t), vec![Action::Navigate(Direction::Left)]);
    // Letting go of Left falls back to the still-held Up.
    assert_eq!(m.handle(release(Button::DPadLeft), t), vec![Action::Navigate(Direction::Up)]);
}

#[test]
fn stick_has_hysteresis() {
    let (mut m, t) = mapper();
    let x = |value| RawEvent::Axis { axis: Axis::LeftX, value };
    assert!(m.handle(x(0.5), t).is_empty(), "below enter threshold");
    assert_eq!(m.handle(x(0.7), t), vec![Action::Navigate(Direction::Right)]);
    assert!(m.handle(x(0.5), t).is_empty(), "between exit and enter keeps the direction, no new move");
    assert!(m.handle(x(0.3), t).is_empty(), "released");
    assert!(m.tick(t + Duration::from_secs(1)).is_empty(), "and no repeat after release");
    assert_eq!(m.handle(x(0.7), t), vec![Action::Navigate(Direction::Right)]);
}

#[test]
fn stick_up_is_positive_y() {
    let (mut m, t) = mapper();
    let y = RawEvent::Axis { axis: Axis::LeftY, value: 0.9 };
    assert_eq!(m.handle(y, t), vec![Action::Navigate(Direction::Up)]);
}

#[test]
fn app_ownership_swallows_everything_but_guide() {
    let (mut m, t) = mapper();
    m.set_owner(InputOwner::App);
    assert!(m.handle(press(Button::South), t).is_empty());
    assert!(m.handle(press(Button::DPadRight), t).is_empty());
    assert!(m.tick(t + Duration::from_secs(1)).is_empty());
    assert_eq!(m.handle(press(Button::Guide), t), vec![Action::MainMenu]);
}

#[test]
fn switching_owner_clears_held_directions() {
    let (mut m, t) = mapper();
    m.handle(press(Button::DPadRight), t);
    m.set_owner(InputOwner::App);
    m.set_owner(InputOwner::Launcher);
    assert!(m.tick(t + Duration::from_secs(1)).is_empty(), "no phantom repeat after returning");
}

fn with_holds() -> (InputMapper, Instant) {
    let (mut m, t) = mapper();
    m.set_hold_rules(vec![
        HoldRule { button: Button::West, after: Duration::from_millis(900) },
        HoldRule { button: Button::North, after: Duration::from_millis(1200) },
    ]);
    m.set_holds_on(true);
    (m, t)
}

#[test]
fn a_held_button_waits_for_the_release_to_do_its_ordinary_thing() {
    let (mut m, t) = with_holds();
    assert_eq!(m.handle(press(Button::West), t), vec![Action::Hold(Button::West, HoldPhase::Started)], "nothing else on press");
    assert_eq!(
        m.handle(release(Button::West), t + Duration::from_millis(200)),
        vec![Action::Hold(Button::West, HoldPhase::Cancelled), Action::Secondary],
        "a tap clears the ring and then acts"
    );
}

#[test]
fn a_completed_hold_is_reported_by_tick_and_the_release_is_silent() {
    let (mut m, t) = with_holds();
    m.handle(press(Button::North), t);
    assert!(m.tick(t + Duration::from_millis(1100)).is_empty());
    assert_eq!(m.tick(t + Duration::from_millis(1200)), vec![Action::Hold(Button::North, HoldPhase::Completed)]);
    assert!(m.handle(release(Button::North), t + Duration::from_millis(1500)).is_empty(), "no Search after a hold");
}

#[test]
fn buttons_without_a_rule_and_a_pad_with_holds_off_act_on_press_as_always() {
    let (mut m, t) = with_holds();
    assert_eq!(m.handle(press(Button::South), t), vec![Action::Confirm]);
    let (mut off, t) = mapper();
    off.set_hold_rules(vec![HoldRule { button: Button::West, after: Duration::from_millis(900) }]);
    assert_eq!(off.handle(press(Button::West), t), vec![Action::Secondary], "rules without holds_on do nothing");
}

#[test]
fn turning_holds_off_mid_hold_clears_the_ring_and_a_directional_tick_still_repeats() {
    let (mut m, t) = with_holds();
    m.handle(press(Button::West), t);
    assert_eq!(m.set_holds_on(false), vec![Action::Hold(Button::West, HoldPhase::Cancelled)]);
    assert!(m.handle(release(Button::West), t).is_empty(), "the late release is ignored");
    m.set_holds_on(true);
    m.handle(press(Button::DPadRight), t);
    assert_eq!(m.tick(t + Duration::from_millis(400)), vec![Action::Navigate(Direction::Right)]);
}

#[test]
fn handing_the_pad_to_an_app_cancels_a_hold() {
    let (mut m, t) = with_holds();
    m.handle(press(Button::West), t);
    m.set_owner(InputOwner::App);
    m.set_owner(InputOwner::Launcher);
    assert!(m.tick(t + Duration::from_secs(5)).is_empty(), "no hold completes after the pad was away");
}

#[test]
fn disconnect_clears_state() {
    let (mut m, t) = mapper();
    m.handle(press(Button::DPadRight), t);
    m.handle(RawEvent::Disconnected, t);
    assert!(m.tick(t + Duration::from_secs(1)).is_empty());
}
