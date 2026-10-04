//! How a surface is presented at each form factor. One table, used by every dialog and menu.
//!
//! * **Form** (inputs, settings, install): a popup on desktop; a full-screen page with a Back
//!   button on phones, handhelds and Deck mode, where there is room to see and reach the fields
//!   and an on-screen keyboard may cover half the screen.
//! * **Confirm** (a sentence and two buttons, no inputs, nothing to scroll): small and centered
//!   everywhere. Full-screen would be all empty space.
//! * **Menu** (a list of choices, possibly cascading): an anchored popover under a pointer;
//!   centered over a darkened screen for touch and gamepad, like Big Picture's option menus.
use crate::metrics::{Density, LayoutClass};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SurfaceKind {
    Form,
    Confirm,
    Menu,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presentation {
    /// Centered card over a dimmed page; the desktop dialog.
    Popup,
    /// Takes the whole screen: header with Back, scrolling body, footer actions.
    FullScreen,
    /// Small centered card (Confirm) or centered list (Menu) over a darkened screen.
    Centered,
    /// A popover next to the thing that opened it. Pointer only.
    Anchored,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceContext {
    pub class: LayoutClass,
    pub density: Density,
    /// Window height in px. A touch device in landscape is short, and a keyboard takes half of it.
    pub height: f32,
}

impl SurfaceContext {
    pub fn new(class: LayoutClass, density: Density, height: f32) -> Self {
        Self { class, density, height }
    }

    /// Deck mode: wide layout, controller density.
    pub fn deck(height: f32) -> Self {
        Self::new(LayoutClass::Wide, Density::Controller, height)
    }

    /// Too small or too coarse for a popup form: phone width, any controller, or a touch device
    /// short enough that a keyboard would leave a popup with no room.
    pub fn is_compact(&self) -> bool {
        self.class == LayoutClass::Phone
            || self.density == Density::Controller
            || (self.density == Density::Touch && self.height < TOUCH_POPUP_MIN_HEIGHT)
    }
}

/// A touch device below this height gets full-screen forms. 900 keeps a 1280x800 handheld and any
/// phone in landscape on the full-screen path while letting a portrait tablet keep popups.
pub const TOUCH_POPUP_MIN_HEIGHT: f32 = 900.;

pub fn presentation(kind: SurfaceKind, ctx: SurfaceContext) -> Presentation {
    match kind {
        SurfaceKind::Form => {
            if ctx.is_compact() {
                Presentation::FullScreen
            } else {
                Presentation::Popup
            }
        }
        SurfaceKind::Confirm => {
            if ctx.density == Density::Controller || ctx.class == LayoutClass::Phone {
                Presentation::Centered
            } else {
                Presentation::Popup
            }
        }
        SurfaceKind::Menu => {
            if ctx.density == Density::Pointer && ctx.class != LayoutClass::Phone {
                Presentation::Anchored
            } else {
                Presentation::Centered
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Density::*;
    use LayoutClass::*;
    use Presentation::*;
    use SurfaceKind::*;

    fn p(kind: SurfaceKind, class: LayoutClass, density: Density, height: f32) -> Presentation {
        presentation(kind, SurfaceContext::new(class, density, height))
    }

    #[test]
    fn desktop_keeps_popups_and_anchored_menus() {
        assert_eq!(p(Form, Wide, Pointer, 800.), Popup);
        assert_eq!(p(Form, Compact, Pointer, 640.), Popup);
        assert_eq!(p(Confirm, Wide, Pointer, 800.), Popup);
        assert_eq!(p(Menu, Wide, Pointer, 800.), Anchored);
    }

    #[test]
    fn deck_mode_is_full_screen_forms_and_centered_everything_else() {
        let deck = SurfaceContext::deck(800.);
        assert_eq!(presentation(Form, deck), FullScreen);
        assert_eq!(presentation(Confirm, deck), Centered);
        assert_eq!(presentation(Menu, deck), Centered);
    }

    #[test]
    fn phones_are_full_screen_forms_even_with_a_mouse() {
        assert_eq!(p(Form, Phone, Pointer, 780.), FullScreen);
        assert_eq!(p(Confirm, Phone, Touch, 780.), Centered);
        assert_eq!(p(Menu, Phone, Pointer, 780.), Centered);
    }

    #[test]
    fn short_touch_screens_go_full_screen_tall_ones_keep_the_popup() {
        // A 1280x800 handheld running the desktop layout with touch density.
        assert_eq!(p(Form, Wide, Touch, 800.), FullScreen);
        assert_eq!(p(Menu, Wide, Touch, 800.), Centered, "fingers get a centered list, never a hover popover");
        // A portrait tablet.
        assert_eq!(p(Form, Compact, Touch, 1200.), Popup);
        assert_eq!(p(Confirm, Compact, Touch, 1200.), Popup);
    }

    #[test]
    fn controller_density_wins_over_size() {
        assert_eq!(p(Form, Wide, Controller, 2160.), FullScreen, "a 4K TV is still a controller UI");
    }
}
