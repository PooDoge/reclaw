//! Deck mode's full-screen pages and cascading menus at each form factor, written to
//! target/snapshots/surface-*.png for review, with layout assertions where a number matters.

use crate::common::*;
use reclaw_input::{Action::*, Direction::*};

const HANDHELD: (f32, f32) = (1280., 800.);
const LANDSCAPE_PHONE: (f32, f32) = (854., 480.);
const PORTRAIT_PHONE: (f32, f32) = (390., 844.);

fn at((w, h): (f32, f32), mount: Mount) -> Mount {
    mount.size(w, h)
}

/// Home -> Dino Rush (not installed) -> game page -> Install.
fn open_install() -> Vec<reclaw_input::Action> {
    vec![Navigate(Right), Navigate(Right), Navigate(Right), Confirm, Confirm]
}

#[test]
fn options_menu_is_centered_and_cascades() {
    let m = Mount::deck().script([Options]);
    m.clone().start().snapshot("surface-options-root");
    m.clone().script([Options, Navigate(Down), Confirm]).start().snapshot("surface-options-add-to");
    at(PORTRAIT_PHONE, m.script([Options, Navigate(Down), Navigate(Down), Confirm])).start().snapshot("surface-options-phone-manage");
}

#[test]
fn uninstall_asks_first() {
    // Options -> Manage -> ... -> Uninstall -> confirm card.
    let script = [Options, Navigate(Down), Navigate(Down), Confirm, Navigate(Down), Navigate(Down), Navigate(Down), Confirm];
    Mount::deck().script(script).start().snapshot("surface-confirm-uninstall");
}

#[test]
fn install_page_at_each_form_factor() {
    let _ = open_install();
    for (name, size) in [("handheld", HANDHELD), ("landscape-phone", LANDSCAPE_PHONE), ("portrait-phone", PORTRAIT_PHONE)] {
        at(size, Mount::deck().script(open_install())).start().snapshot(&format!("surface-install-{name}"));
    }
}

#[test]
fn settings_page_two_pane_and_drill_down() {
    let to_settings = [MainMenu, Navigate(Down), Navigate(Down), Navigate(Down), Navigate(Down), Confirm];
    at(HANDHELD, Mount::deck().script(to_settings)).start().snapshot("surface-settings-two-pane");
    at(PORTRAIT_PHONE, Mount::deck().script(to_settings)).start().snapshot("surface-settings-phone-list");
}
