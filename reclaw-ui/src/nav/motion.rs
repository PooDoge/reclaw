use freya::prelude::*;

use super::transition::{Role, Spec, TransitionStyle};

/// Where a page is in its transition, for pages that stage their own entrance (a game page zooming
/// its banner and staggering its artwork). A page at rest reads `PageMotion::REST`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PageMotion {
    pub role: Role,
    /// 0 when the transition starts, 1 when it ends.
    pub progress: f32,
    pub style: TransitionStyle,
}

impl PageMotion {
    pub const REST: PageMotion = PageMotion { role: Role::Entering, progress: 1., style: TransitionStyle::None };

    pub(super) fn new(role: Role, progress: f32, spec: &Spec) -> Self {
        Self { role, progress, style: spec.style }
    }

    /// `progress` mapped to a window: 0 before `start`, 1 after `end`. Staggers parts of a page:
    /// `motion.between(0.0, 0.6)` for the banner, `motion.between(0.3, 1.0)` for what follows.
    pub fn between(&self, start: f32, end: f32) -> f32 {
        if end <= start {
            return if self.progress >= end { 1. } else { 0. };
        }
        ((self.progress - start) / (end - start)).clamp(0., 1.)
    }

    /// Opacity and downward offset for a part of the page that rises into place during a hero
    /// entrance, over the `start..end` window of the transition. At rest it is `(1, 0)`.
    pub fn rise(&self, start: f32, end: f32, distance: f32) -> (f32, f32) {
        if !self.is_hero_entrance() {
            return (1., 0.);
        }
        let t = self.between(start, end);
        (t, (1. - t) * distance)
    }

    /// Scale for a part that settles from `1 + amount` down to 1 during a hero entrance (a banner
    /// easing in). 1 at rest.
    pub fn settle(&self, start: f32, end: f32, amount: f32) -> f32 {
        if !self.is_hero_entrance() {
            return 1.;
        }
        1. + amount * (1. - self.between(start, end))
    }

    /// Whether this page should play its own entrance: it is the one arriving, and asked for one.
    pub fn is_hero_entrance(&self) -> bool {
        self.role == Role::Entering && self.style == TransitionStyle::Hero && self.progress < 1.
    }
}

/// The motion of the page this component is part of. Safe anywhere: outside a stage it reports rest.
pub fn use_page_motion() -> PageMotion {
    match try_consume_context::<State<PageMotion>>() {
        Some(state) => *state.read(),
        None => PageMotion::REST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(progress: f32) -> PageMotion {
        PageMotion { role: Role::Entering, progress, style: TransitionStyle::Hero }
    }

    #[test]
    fn between_maps_a_window_of_the_transition_to_zero_one() {
        assert_eq!(at(0.).between(0.2, 0.6), 0.);
        assert!((at(0.4).between(0.2, 0.6) - 0.5).abs() < 1e-5);
        assert_eq!(at(1.).between(0.2, 0.6), 1.);
        assert_eq!(at(0.5).between(0.5, 0.5), 1., "an empty window is a step");
    }

    #[test]
    fn parts_rise_and_settle_only_during_a_hero_entrance() {
        assert_eq!(at(0.).rise(0.2, 0.6, 20.), (0., 20.));
        assert_eq!(at(1.).rise(0.2, 0.6, 20.), (1., 0.));
        assert_eq!(at(0.).settle(0., 1., 0.1), 1.1);
        let slide = PageMotion { style: TransitionStyle::Slide, ..at(0.) };
        assert_eq!((slide.rise(0.2, 0.6, 20.), slide.settle(0., 1., 0.1)), ((1., 0.), 1.), "other styles leave the parts alone");
        assert_eq!(PageMotion::REST.rise(0., 1., 20.), (1., 0.));
    }

    #[test]
    fn only_an_arriving_hero_page_plays_an_entrance() {
        assert!(at(0.3).is_hero_entrance());
        assert!(!at(1.).is_hero_entrance());
        assert!(!PageMotion { role: Role::Leaving, ..at(0.3) }.is_hero_entrance());
        assert!(!PageMotion { style: TransitionStyle::Slide, ..at(0.3) }.is_hero_entrance());
        assert!(!PageMotion::REST.is_hero_entrance());
    }
}
