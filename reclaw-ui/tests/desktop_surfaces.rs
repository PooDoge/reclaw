//! The desktop interface's dialogs and menus at each form factor: popups and anchored menus under
//! a pointer, full-screen pages and centered menus on phones and touch handhelds.
mod common;

use common::*;
use reclaw_ui::{
    metrics::{Density, LayoutClass},
    shell::DevOverrides,
};

fn dev(layout: Option<LayoutClass>, density: Option<Density>) -> DevOverrides {
    DevOverrides { layout, density, ..DevOverrides::default() }
}

/// Select Dino Rush (not installed) and press its Install button.
fn open_install(s: &mut Session, phone: bool) {
    s.click_label("Dino Rush");
    if phone {
        // The phone grid opens the game page on the first press.
        assert!(s.has_label("Library"), "the phone page has its back button: {:?}", s.labels());
    }
    s.click_label("Install");
}

#[test]
fn the_install_dialog_is_a_popup_on_a_wide_desktop() {
    let mut s = Mount::desktop().start();
    open_install(&mut s, false);
    assert!(s.has_label("Install Dino Rush"), "{:?}", s.labels());
    assert!(s.has_label("Cancel"), "popup actions");
    assert!(!s.has_label("Back"), "a popup has no Back button: {:?}", s.labels());
    s.snapshot("desktop-install-popup");
}

#[test]
fn the_install_dialog_is_a_full_screen_page_on_a_phone() {
    let mut s = Mount::desktop().size(390., 780.).start();
    open_install(&mut s, true);
    assert!(s.has_label("Install Dino Rush"), "{:?}", s.labels());
    assert!(s.has_label("Back"), "full-screen forms have a Back button");
    s.snapshot("desktop-install-phone");

    s.click_label("Back");
    assert!(!s.has_label("Install Dino Rush"), "Back closes the form");
}

#[test]
fn a_short_touch_handheld_gets_the_full_screen_page_even_though_it_is_wide() {
    let mut s = Mount::desktop().size(1280., 800.).dev(dev(None, Some(Density::Touch))).start();
    open_install(&mut s, false);
    assert!(s.has_label("Back"), "touch at 800px tall is full screen: {:?}", s.labels());
    s.snapshot("desktop-install-touch-handheld");
}

#[test]
fn a_tall_tablet_keeps_the_popup() {
    let mut s = Mount::desktop().size(1280., 1000.).dev(dev(None, Some(Density::Touch))).start();
    open_install(&mut s, false);
    assert!(!s.has_label("Back") && s.has_label("Cancel"), "{:?}", s.labels());
}

#[test]
fn the_keyboard_hides_the_footer_of_a_full_screen_form() {
    // A landscape handheld: compact layout, touch density, 480px tall.
    let mut s = Mount::desktop().size(854., 480.).dev(dev(None, Some(Density::Touch))).start();
    s.wheel(500., 300., -400.); // the capsule grid is below the hero
    s.click_label("Dino Rush");
    s.wheel(500., 300., 400.); // back up to the hero's Install button
    s.click_label("Install");
    s.set_keyboard(216.);
    assert!(s.has_label("Install location"), "{:?}", s.labels());
    assert!(!s.has_label("Cancel"), "actions hide while the keyboard is up");
    let (_, bottom) = s.label_span("Install location").unwrap();
    assert!(bottom <= 480. - 216., "the field is above the keyboard: {bottom}");
    s.set_keyboard(0.);
    assert!(s.has_label("Cancel") || s.has_label("Install"), "actions return: {:?}", s.labels());
    s.snapshot("desktop-install-phone-landscape");
}

#[test]
fn manage_opens_a_menu_anchored_at_the_pointer_on_desktop_and_centered_on_touch() {
    // Starfall 64 is selected and installed, so its hero has a Manage button.
    let mut pointer = Mount::desktop().start();
    let (button_left, _, button_right, _) = pointer.label_box("Manage").expect("the hero's Manage button");
    pointer.click_label("Manage");
    assert!(pointer.has_label("Add to favorites"), "{:?}", pointer.labels());
    let (menu_left, ..) = pointer.label_box("Add to favorites").unwrap();
    // The button sits at the right edge, so the menu slides left to stay on screen.
    assert!(menu_left < button_right && menu_left > button_left - 250., "menu {menu_left} vs button {button_left}..{button_right}");
    pointer.snapshot("desktop-manage-anchored");

    let mut touch = Mount::desktop().size(1280., 800.).dev(dev(None, Some(Density::Touch))).start();
    touch.click_label("Manage");
    let (menu_left, _, menu_right, _) = touch.label_box("Add to favorites").expect("the menu is open");
    let middle = (menu_left + menu_right) / 2.;
    assert!((middle - 640.).abs() < 220., "touch centers the menu: label spans {menu_left}..{menu_right}");
    touch.snapshot("desktop-manage-centered-touch");
}

#[test]
fn uninstall_from_the_desktop_menu_asks_first() {
    let mut s = Mount::desktop().start();
    s.click_label("Manage");
    s.click_label("Manage"); // the row of the same name, now on top, opens its submenu
    s.click_label("Uninstall");
    assert!(s.has_label("Uninstall Starfall 64?"), "{:?}", s.labels());
    assert!(s.take_effects().is_empty(), "nothing is sent before the answer");
    s.click_label("Cancel");
    assert!(!s.has_label("Uninstall Starfall 64?"));

    s.click_label("Manage");
    s.click_label("Manage");
    s.click_label("Uninstall");
    s.click_label("Uninstall");
    assert_eq!(s.take_effects(), vec![reclaw_ui::effect::Effect::Uninstall(1)]);
}
