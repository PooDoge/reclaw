use super::{
    config::Spec,
    style::{Direction, TransitionStyle},
};

/// Which page of a transition a frame is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Entering,
    Leaving,
}

/// How one page is drawn at one moment of a transition. `dx`/`dy` are logical px offsets.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Frame {
    pub opacity: f32,
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
}

impl Frame {
    pub const REST: Frame = Frame { opacity: 1., dx: 0., dy: 0., scale: 1. };
    const GONE: Frame = Frame { opacity: 0., dx: 0., dy: 0., scale: 1. };
}

/// The frame of `role` when the transition `spec` is `progress` of the way through (0 at the start,
/// 1 at the end; already eased). Pure: the stage animates `progress` and applies this.
pub fn frame(spec: &Spec, role: Role, progress: f32) -> Frame {
    let p = progress.clamp(0., 1.);
    let d = spec.distance;
    let forward = spec.direction != Direction::Back;
    // +1 when new pages arrive from the right/below, -1 when the way back reveals them.
    let s = if forward { 1. } else { -1. };
    let (enter, leave) = (p, 1. - p);
    match (spec.style, role) {
        (TransitionStyle::None, Role::Entering) => Frame::REST,
        (TransitionStyle::None, Role::Leaving) => Frame::GONE,
        (TransitionStyle::Fade, Role::Entering) => Frame { opacity: enter, ..Frame::REST },
        (TransitionStyle::Fade, Role::Leaving) => Frame { opacity: leave, ..Frame::REST },
        (TransitionStyle::Slide, Role::Entering) => Frame { opacity: enter, dx: s * leave * d, ..Frame::REST },
        (TransitionStyle::Slide, Role::Leaving) => Frame { opacity: leave, dx: -s * p * d * 0.5, ..Frame::REST },
        (TransitionStyle::Rise, Role::Entering) => Frame { opacity: enter, dy: if forward { leave * d } else { 0. }, ..Frame::REST },
        (TransitionStyle::Rise, Role::Leaving) => Frame { opacity: leave, dy: if forward { 0. } else { p * d }, ..Frame::REST },
        (TransitionStyle::Zoom, Role::Entering) => {
            Frame { opacity: enter, scale: if forward { 0.96 + 0.04 * p } else { 1.02 - 0.02 * p }, ..Frame::REST }
        }
        (TransitionStyle::Zoom, Role::Leaving) => {
            Frame { opacity: leave, scale: if forward { 1. + 0.02 * p } else { 1. - 0.04 * p }, ..Frame::REST }
        }
        // The page stages its own parts; the stage only fades it up and settles it from slightly below.
        (TransitionStyle::Hero, Role::Entering) => {
            Frame { opacity: p.sqrt(), dy: if forward { leave * d * 0.5 } else { 0. }, ..Frame::REST }
        }
        (TransitionStyle::Hero, Role::Leaving) => {
            Frame { opacity: (1. - p * 1.5).max(0.), dy: if forward { 0. } else { p * d * 0.5 }, ..Frame::REST }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::nav::transition::Easing;

    const STYLES: [TransitionStyle; 6] = [
        TransitionStyle::None,
        TransitionStyle::Fade,
        TransitionStyle::Slide,
        TransitionStyle::Rise,
        TransitionStyle::Zoom,
        TransitionStyle::Hero,
    ];

    fn spec(style: TransitionStyle, direction: Direction) -> Spec {
        Spec { style, direction, duration: Duration::from_millis(240), distance: 48., easing: Easing::ExpoOut }
    }

    #[test]
    fn at_the_end_the_new_page_is_at_rest_and_the_old_one_is_gone() {
        for style in STYLES {
            for direction in [Direction::Forward, Direction::Back, Direction::Replace] {
                let s = spec(style, direction);
                assert_eq!(frame(&s, Role::Entering, 1.), Frame::REST, "{style:?} {direction:?}");
                assert_eq!(frame(&s, Role::Leaving, 1.).opacity, 0., "{style:?} {direction:?}");
            }
        }
    }

    #[test]
    fn at_the_start_the_old_page_is_at_rest_and_the_new_one_is_hidden() {
        for style in STYLES.into_iter().filter(|s| *s != TransitionStyle::None) {
            let s = spec(style, Direction::Forward);
            assert_eq!(frame(&s, Role::Leaving, 0.), Frame::REST, "{style:?}");
            assert_eq!(frame(&s, Role::Entering, 0.).opacity, 0., "{style:?}");
        }
    }

    #[test]
    fn opacity_never_leaves_zero_to_one_and_moves_one_way() {
        for style in STYLES.into_iter().filter(|s| *s != TransitionStyle::None) {
            for direction in [Direction::Forward, Direction::Back] {
                let s = spec(style, direction);
                let steps: Vec<f32> = (0..=10).map(|i| i as f32 / 10.).collect();
                let entering: Vec<f32> = steps.iter().map(|p| frame(&s, Role::Entering, *p).opacity).collect();
                let leaving: Vec<f32> = steps.iter().map(|p| frame(&s, Role::Leaving, *p).opacity).collect();
                assert!(
                    entering.windows(2).all(|w| w[0] <= w[1]) && entering.iter().all(|o| (0.0..=1.0).contains(o)),
                    "{style:?} {entering:?}"
                );
                assert!(
                    leaving.windows(2).all(|w| w[0] >= w[1]) && leaving.iter().all(|o| (0.0..=1.0).contains(o)),
                    "{style:?} {leaving:?}"
                );
            }
        }
    }

    #[test]
    fn slides_come_from_opposite_sides_going_forward_and_back() {
        let forward = frame(&spec(TransitionStyle::Slide, Direction::Forward), Role::Entering, 0.);
        let back = frame(&spec(TransitionStyle::Slide, Direction::Back), Role::Entering, 0.);
        assert_eq!(forward.dx, 48.);
        assert_eq!(back.dx, -48.);
    }

    #[test]
    fn progress_outside_zero_to_one_is_clamped() {
        let s = spec(TransitionStyle::Slide, Direction::Forward);
        assert_eq!(frame(&s, Role::Entering, 3.), frame(&s, Role::Entering, 1.));
        assert_eq!(frame(&s, Role::Entering, -1.), frame(&s, Role::Entering, 0.));
    }
}
