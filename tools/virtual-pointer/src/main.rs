//! A virtual pointer that stays alive, driven by lines on stdin: `move X Y`, `down`, `up`, `wheel N`.
//! Coordinates are in a 1600x1000 space mapped over the output. Test tooling for `scripts/wayland-smoke.sh`:
//! a headless wlroots compositor has no input device, so its seat offers clients no pointer until one exists,
//! and `wlrctl` makes one only for the length of a single command.
//!
//! Needs a compositor that implements `zwlr_virtual_pointer_manager_v1` (sway, labwc, Hyprland; not GNOME).
use std::{io::BufRead, time::Instant};

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_pointer::{Axis, ButtonState}, wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

struct State;
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(_: &mut Self, _: &wl_seat::WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<ZwlrVirtualPointerManagerV1, ()> for State {
    fn event(_: &mut Self, _: &ZwlrVirtualPointerManagerV1, _: <ZwlrVirtualPointerManagerV1 as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<ZwlrVirtualPointerV1, ()> for State {
    fn event(_: &mut Self, _: &ZwlrVirtualPointerV1, _: <ZwlrVirtualPointerV1 as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=1, ())?;
    let manager: ZwlrVirtualPointerManagerV1 = globals.bind(&qh, 1..=2, ())?;
    let pointer = manager.create_virtual_pointer(Some(&seat), &qh, ());
    queue.roundtrip(&mut State)?;
    let start = Instant::now();
    eprintln!("vptr ready");
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let t = start.elapsed().as_millis() as u32;
        let mut words = line.split_whitespace();
        match words.next() {
            Some("move") => {
                let x: u32 = words.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                let y: u32 = words.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                pointer.motion_absolute(t, x, y, 1600, 1000);
                pointer.frame();
            }
            Some("down") => { pointer.button(t, 0x110, ButtonState::Pressed); pointer.frame(); }
            Some("up") => { pointer.button(t, 0x110, ButtonState::Released); pointer.frame(); }
            Some("wheel") => {
                let n: f64 = words.next().and_then(|v| v.parse().ok()).unwrap_or(1.);
                pointer.axis_discrete(t, Axis::VerticalScroll, n * 15., n as i32);
                pointer.frame();
            }
            _ => {}
        }
        queue.roundtrip(&mut State)?;
    }
    Ok(())
}
