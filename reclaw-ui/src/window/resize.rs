//! Where the resize bands go along the edges of a window that has no border of its own. Pure, so the
//! geometry is tested with plain numbers; `bands` (the component) draws them and `platform` starts the
//! resize.
//!
//! Sizes are in the UI's layout units, the ones `Size::px` and `Position` use, which are the window's
//! physical pixels divided by the display scale and the UI scale. Freya's own bands take the window's
//! physical size and place themselves in layout units, so on a scaled display their right and bottom
//! bands lay outside the window and only the left and top edges answered.

/// A side or corner of the window a resize can start from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    North,
    South,
    West,
    East,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}

/// How far into the window a band reaches. With no border there is nothing to aim at, so this is wider
/// than a hairline; GNOME's own windows have about this much grab area around them.
pub const THICKNESS: f32 = 8.;

/// One band: the edge it starts a resize from and where it sits, from the window's top left.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Band {
    pub edge: Edge,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// The eight bands for a window of `(width, height)`: four sides, and a corner square twice as wide as
/// a side at each corner. They cover the outer `thickness` of the window and never overlap or leave it.
pub fn bands((w, h): (f32, f32), thickness: f32) -> Vec<Band> {
    let (w, h) = (w.max(0.), h.max(0.));
    // A corner never takes more than half of a side, so a tiny window still has all eight.
    let corner = (thickness * 2.).min(w / 2.).min(h / 2.);
    let band = |edge, left, top, width, height| Band { edge, left, top, width, height };
    let (span_x, span_y) = ((w - corner * 2.).max(0.), (h - corner * 2.).max(0.));
    let (side_x, side_y) = (thickness.min(w / 2.), thickness.min(h / 2.));
    vec![
        band(Edge::North, corner, 0., span_x, side_y),
        band(Edge::South, corner, h - side_y, span_x, side_y),
        band(Edge::West, 0., corner, side_x, span_y),
        band(Edge::East, w - side_x, corner, side_x, span_y),
        band(Edge::NorthWest, 0., 0., corner, corner),
        band(Edge::NorthEast, w - corner, 0., corner, corner),
        band(Edge::SouthWest, 0., h - corner, corner, corner),
        band(Edge::SouthEast, w - corner, h - corner, corner, corner),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: [(f32, f32); 6] = [(900., 560.), (1280., 800.), (1067., 667.), (640., 400.), (20., 14.), (0., 0.)];

    fn of(list: &[Band], edge: Edge) -> Band {
        *list.iter().find(|b| b.edge == edge).expect("every edge has a band")
    }

    #[test]
    fn every_edge_has_one_band_and_the_far_ones_touch_the_far_sides_of_this_size() {
        for (w, h) in SIZES.into_iter().filter(|(w, h)| *w >= 100. && *h >= 100.) {
            let list = bands((w, h), THICKNESS);
            assert_eq!(list.len(), 8);
            // The right and bottom bands end at the window's right and bottom, whatever the size is: the
            // bug was that they ended at a size in the wrong units.
            assert_eq!(of(&list, Edge::East).left + of(&list, Edge::East).width, w, "{w}x{h}");
            assert_eq!(of(&list, Edge::South).top + of(&list, Edge::South).height, h, "{w}x{h}");
            assert_eq!(of(&list, Edge::SouthEast).left + of(&list, Edge::SouthEast).width, w);
            assert_eq!(of(&list, Edge::SouthEast).top + of(&list, Edge::SouthEast).height, h);
            assert_eq!(of(&list, Edge::North).top, 0.);
            assert_eq!(of(&list, Edge::West).left, 0.);
        }
    }

    #[test]
    fn no_band_leaves_the_window_or_overlaps_another() {
        for size in SIZES {
            let list = bands(size, THICKNESS);
            for b in &list {
                assert!(b.left >= 0. && b.top >= 0. && b.width >= 0. && b.height >= 0., "{size:?} {b:?}");
                assert!(b.left + b.width <= size.0 + 0.001 && b.top + b.height <= size.1 + 0.001, "{size:?} {b:?} leaves the window");
            }
            for (i, a) in list.iter().enumerate() {
                for b in &list[i + 1..] {
                    let overlap_x = (a.left + a.width).min(b.left + b.width) - a.left.max(b.left);
                    let overlap_y = (a.top + a.height).min(b.top + b.height) - a.top.max(b.top);
                    assert!(overlap_x <= 0.001 || overlap_y <= 0.001, "{size:?}: {:?} overlaps {:?}", a.edge, b.edge);
                }
            }
        }
    }

    #[test]
    fn a_corner_is_twice_as_wide_as_a_side_so_it_is_easy_to_hit() {
        let list = bands((900., 560.), THICKNESS);
        assert_eq!(of(&list, Edge::East).width, THICKNESS);
        assert_eq!(of(&list, Edge::NorthWest).width, THICKNESS * 2.);
        assert_eq!(of(&list, Edge::NorthWest).height, THICKNESS * 2.);
    }
}
