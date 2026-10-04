use freya::prelude::*;

use super::Nav;

/// Back and forward from the devices that have them: the mouse's side buttons, Alt+Left and
/// Alt+Right, and the browser-style Back and Forward keys some keyboards send. `escape` also makes
/// Escape go back; Deck mode passes `false` because its reducer already turns Escape, like the pad's
/// B button, into Back (after closing its own menus).
pub trait NavInputExt {
    fn nav_input(self, nav: Nav, escape: bool) -> Self;
}

impl NavInputExt for Rect {
    fn nav_input(self, nav: Nav, escape: bool) -> Self {
        self.on_global_pointer_up(move |e: Event<PointerEventData>| match e.button() {
            Some(MouseButton::Back) => {
                nav.back();
            }
            Some(MouseButton::Forward) => {
                nav.forward();
            }
            _ => {}
        })
        .on_global_key_down(move |e: Event<KeyboardEventData>| {
            let alt = e.modifiers.contains(Modifiers::ALT);
            match &e.key {
                Key::Named(NamedKey::BrowserBack) => {
                    nav.back();
                }
                Key::Named(NamedKey::Escape) if escape => {
                    nav.back();
                }
                Key::Named(NamedKey::BrowserForward) => {
                    nav.forward();
                }
                Key::Named(NamedKey::ArrowLeft) if alt => {
                    nav.back();
                }
                Key::Named(NamedKey::ArrowRight) if alt => {
                    nav.forward();
                }
                _ => {}
            }
        })
    }
}
