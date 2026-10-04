use freya::{animation::*, prelude::*};

use crate::{metrics::*, prelude::*};

fn scale_of(on: bool, target: f32) -> f32 {
    if on { target } else { 1. }
}

/// Focus decoration for any single element: scale, accent ring, glow. Wrapping instead of styling
/// the child keeps Freya's own Button (which draws its own border) untouched.
#[derive(Clone, PartialEq)]
pub struct FocusFrame {
    focused: bool,
    scale: f32,
    radius: f32,
    child: Element,
}

impl FocusFrame {
    pub fn new(child: impl IntoElement, focused: bool) -> Self {
        Self {
            focused,
            scale: DECK_BUTTON_SCALE,
            radius: RADIUS_MD,
            child: child.into_element(),
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

impl Component for FocusFrame {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let target = self.scale;
        let anim = use_animation_transition(self.focused, move |from: bool, to: bool| {
            AnimNum::new(scale_of(from, target), scale_of(to, target))
                .time(120)
                .ease(Ease::Out)
        });
        let s = anim.read().value();
        rect()
            .corner_radius(self.radius + DECK_FOCUS_RING)
            .scale((s, s))
            .border(
                Border::new()
                    .fill(if self.focused {
                        t.accent
                    } else {
                        crate::components::CLEAR
                    })
                    .width(DECK_FOCUS_RING)
                    .alignment(BorderAlignment::Outer),
            )
            .maybe(self.focused, |el| {
                el.shadow(Shadow::new().blur(24.).spread(2.).color(t.focus_glow))
            })
            .child(self.child.clone())
    }
}
