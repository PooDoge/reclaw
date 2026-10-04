//! Scroll math shared by rendering and tests. The focus rectangles in `state` use the same
//! constants, so what is drawn and what is focusable cannot drift apart.
use crate::metrics::*;

/// Horizontal offset that keeps the focused tile centered in a shelf of `count` tiles, clamped so
/// the row never scrolls past its ends. `viewport` is the visible width of the shelf.
pub fn shelf_offset(count: usize, focus: usize, viewport: f32) -> f32 {
    if count == 0 {
        return 0.;
    }
    let step = DECK_TILE_W + DECK_TILE_GAP;
    let content = count as f32 * step - DECK_TILE_GAP;
    if content <= viewport {
        return 0.;
    }
    (focus as f32 * step - (viewport - DECK_TILE_W) / 2.).clamp(0., content - viewport)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_rows_do_not_scroll() {
        assert_eq!(shelf_offset(3, 2, 1000.), 0.);
        assert_eq!(shelf_offset(0, 0, 1000.), 0.);
    }

    #[test]
    fn long_rows_center_the_focus_and_clamp() {
        let step = DECK_TILE_W + DECK_TILE_GAP;
        assert_eq!(shelf_offset(20, 0, 1000.), 0.);
        let mid = shelf_offset(20, 10, 1000.);
        assert!((mid - (10. * step - (1000. - DECK_TILE_W) / 2.)).abs() < 0.01);
        let end = shelf_offset(20, 19, 1000.);
        assert!(
            (end - (20. * step - DECK_TILE_GAP - 1000.)).abs() < 0.01,
            "clamped to the last tile"
        );
    }
}
