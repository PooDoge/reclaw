//! gilrs-backed hardware reader. It runs on its own thread (`Gilrs` is not `Send`) and publishes
//! [`InputMessage`]s on a channel the UI drains.
//!
//! gilrs normalizes stick Y so that up is positive (it flips axes on platforms that report them
//! reversed), which is what [`RawEvent`] expects.
//!
//! **Not exercised against real hardware in the development container** (no `/dev/input`): it is
//! compile-checked, and everything it feeds (the mapper, glyphs, spatial focus) is unit-tested.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gilrs::{Axis as GAxis, Button as GButton, EventType, Gilrs, PowerInfo};

use crate::{
    action::{Action, ActionMap, Axis, Button, RawEvent},
    controller::{ControllerInfo, PowerState},
    hold::HoldRule,
    mapper::{InputMapper, InputOwner, MapperConfig},
};

#[derive(Clone, PartialEq, Debug)]
pub enum InputMessage {
    Action(Action),
    Connected(ControllerInfo),
    Disconnected {
        id: u32,
    },
    /// gilrs could not start (no udev, no permission). The UI keeps working with keyboard/mouse.
    Unavailable(String),
}

/// Lets the app change who owns the pad without touching the reader thread's state.
#[derive(Clone)]
pub struct InputHandle {
    owner: Arc<AtomicU8>,
    holds: Arc<AtomicBool>,
}

impl InputHandle {
    pub fn set_owner(&self, owner: InputOwner) {
        self.owner.store(owner as u8, Ordering::SeqCst);
    }

    /// Turn the hold rules on while something on screen offers a hold, and off again after: a held
    /// button acts on release instead of press, which is only wanted while a hold can happen.
    pub fn set_holds(&self, on: bool) {
        self.holds.store(on, Ordering::SeqCst);
    }
}

/// Start the reader with no hold rules.
pub fn spawn(map: ActionMap) -> std::io::Result<(InputHandle, UnboundedReceiver<InputMessage>)> {
    spawn_with(map, Vec::new())
}

/// Start the reader. `holds` lists the buttons that can be held (they act as holds only while
/// [`InputHandle::set_holds`] is on). Fails only if the OS refuses to start a thread.
pub fn spawn_with(map: ActionMap, holds: Vec<HoldRule>) -> std::io::Result<(InputHandle, UnboundedReceiver<InputMessage>)> {
    let (tx, rx) = unbounded();
    let owner = Arc::new(AtomicU8::new(InputOwner::Launcher as u8));
    let hold_flag = Arc::new(AtomicBool::new(false));
    let handle = InputHandle { owner: owner.clone(), holds: hold_flag.clone() };
    thread::Builder::new().name("reclaw-gamepad".into()).spawn(move || run(map, holds, owner, hold_flag, tx))?;
    Ok((handle, rx))
}

fn run(map: ActionMap, holds: Vec<HoldRule>, owner: Arc<AtomicU8>, hold_flag: Arc<AtomicBool>, tx: UnboundedSender<InputMessage>) {
    let mut gilrs = match Gilrs::new() {
        Ok(g) => g,
        Err(e) => {
            let _ = tx.unbounded_send(InputMessage::Unavailable(e.to_string()));
            return;
        }
    };
    let mut mapper = InputMapper::new(map, MapperConfig::default());
    mapper.set_hold_rules(holds);
    let mut holds_on = false;

    // Announce pads that were already plugged in.
    for (id, pad) in gilrs.gamepads() {
        if tx.unbounded_send(InputMessage::Connected(info(usize::from(id) as u32, &pad))).is_err() {
            return;
        }
    }

    loop {
        let wanted = if owner.load(Ordering::SeqCst) == InputOwner::App as u8 { InputOwner::App } else { InputOwner::Launcher };
        mapper.set_owner(wanted);

        let now = Instant::now();
        let mut out = Vec::new();
        let wanted_holds = hold_flag.load(Ordering::SeqCst);
        if wanted_holds != holds_on {
            holds_on = wanted_holds;
            out.extend(mapper.set_holds_on(holds_on).into_iter().map(InputMessage::Action));
        }
        // Wake at least every 8ms so held directions repeat on time.
        if let Some(event) = gilrs.next_event_blocking(Some(Duration::from_millis(8))) {
            let id = usize::from(event.id) as u32;
            match event.event {
                EventType::Connected => {
                    let pad = gilrs.gamepad(event.id);
                    let _ = tx.unbounded_send(InputMessage::Connected(info(id, &pad)));
                }
                EventType::Disconnected => {
                    out.extend(mapper.handle(RawEvent::Disconnected, now).into_iter().map(InputMessage::Action));
                    let _ = tx.unbounded_send(InputMessage::Disconnected { id });
                }
                EventType::ButtonPressed(b, _) => push_button(&mut mapper, &mut out, b, true, now),
                EventType::ButtonReleased(b, _) => push_button(&mut mapper, &mut out, b, false, now),
                EventType::AxisChanged(a, value, _) => {
                    if let Some(axis) = map_axis(a) {
                        out.extend(mapper.handle(RawEvent::Axis { axis, value }, now).into_iter().map(InputMessage::Action));
                    }
                }
                _ => {}
            }
        }
        out.extend(mapper.tick(Instant::now()).into_iter().map(InputMessage::Action));
        for message in out {
            if tx.unbounded_send(message).is_err() {
                return; // The UI is gone.
            }
        }
    }
}

fn push_button(mapper: &mut InputMapper, out: &mut Vec<InputMessage>, b: GButton, pressed: bool, now: Instant) {
    if let Some(button) = map_button(b) {
        out.extend(mapper.handle(RawEvent::Button { button, pressed }, now).into_iter().map(InputMessage::Action));
    }
}

fn info(id: u32, pad: &gilrs::Gamepad<'_>) -> ControllerInfo {
    let mut info = ControllerInfo::new(id, pad.name(), pad.vendor_id(), pad.product_id());
    info.power = match pad.power_info() {
        PowerInfo::Unknown => PowerState::Unknown,
        PowerInfo::Wired => PowerState::Wired,
        PowerInfo::Discharging(p) => PowerState::Discharging(p),
        PowerInfo::Charging(p) => PowerState::Charging(p),
        PowerInfo::Charged => PowerState::Full,
    };
    info
}

fn map_button(b: GButton) -> Option<Button> {
    Some(match b {
        GButton::South => Button::South,
        GButton::East => Button::East,
        GButton::West => Button::West,
        GButton::North => Button::North,
        GButton::LeftTrigger => Button::LeftBumper,
        GButton::RightTrigger => Button::RightBumper,
        GButton::LeftTrigger2 => Button::LeftTrigger,
        GButton::RightTrigger2 => Button::RightTrigger,
        GButton::Select => Button::Select,
        GButton::Start => Button::Start,
        GButton::Mode => Button::Guide,
        GButton::LeftThumb => Button::LeftStick,
        GButton::RightThumb => Button::RightStick,
        GButton::DPadUp => Button::DPadUp,
        GButton::DPadDown => Button::DPadDown,
        GButton::DPadLeft => Button::DPadLeft,
        GButton::DPadRight => Button::DPadRight,
        _ => return None,
    })
}

fn map_axis(a: GAxis) -> Option<Axis> {
    Some(match a {
        GAxis::LeftStickX => Axis::LeftX,
        GAxis::LeftStickY => Axis::LeftY,
        GAxis::RightStickX => Axis::RightX,
        GAxis::RightStickY => Axis::RightY,
        _ => return None,
    })
}
