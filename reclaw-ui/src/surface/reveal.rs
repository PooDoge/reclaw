//! Keyboard avoidance: keep a focused field visible when an on-screen keyboard covers the bottom
//! of the window.
//!
//! All positions are in the scrolling content's own coordinates, and the scroll value is how far
//! the content has moved up (0 = top). Freya's controller stores the negative of this.

/// Height of the strip a scrolling body can actually show: the window minus the header minus the
/// keyboard (the footer is hidden while the keyboard is up, so it is not subtracted).
pub fn visible_height(window_h: f32, header_h: f32, footer_h: f32, keyboard_inset: f32) -> f32 {
    let footer = if keyboard_inset > 0. { 0. } else { footer_h };
    (window_h - header_h - footer - keyboard_inset).max(0.)
}

/// The new scroll value that makes `[node_top, node_bottom]` visible with `margin` to spare,
/// moving as little as possible. A node taller than the viewport is aligned to its top.
pub fn scroll_to_reveal(scroll: f32, node_top: f32, node_bottom: f32, viewport: f32, margin: f32) -> f32 {
    let mut scroll = scroll.max(0.);
    if node_bottom - node_top + 2. * margin >= viewport {
        return (node_top - margin).max(0.);
    }
    if node_top - margin < scroll {
        scroll = node_top - margin;
    } else if node_bottom + margin > scroll + viewport {
        scroll = node_bottom + margin - viewport;
    }
    scroll.max(0.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_shrinks_the_viewport_and_hides_the_footer() {
        assert_eq!(visible_height(800., 64., 72., 0.), 664.);
        assert_eq!(visible_height(800., 64., 72., 320.), 416., "footer is hidden, keyboard subtracted");
        assert_eq!(visible_height(300., 64., 72., 320.), 0., "never negative");
    }

    #[test]
    fn visible_nodes_do_not_move() {
        assert_eq!(scroll_to_reveal(100., 150., 210., 400., 16.), 100.);
    }

    #[test]
    fn a_node_below_the_fold_scrolls_just_enough() {
        // Viewport shows 0..400; the field is at 520..580.
        assert_eq!(scroll_to_reveal(0., 520., 580., 400., 16.), 196.);
    }

    #[test]
    fn a_node_above_the_fold_scrolls_back_up() {
        assert_eq!(scroll_to_reveal(500., 120., 180., 400., 16.), 104.);
    }

    #[test]
    fn keyboard_case_end_to_end() {
        // Landscape phone: 360px tall window, 48px header, a 200px keyboard.
        let viewport = visible_height(360., 48., 64., 200.);
        assert_eq!(viewport, 112.);
        // Field at 300..356 in content: must end above the keyboard edge.
        let scroll = scroll_to_reveal(0., 300., 356., viewport, 12.);
        assert!(356. - scroll + 12. <= viewport, "bottom of the field is above the keyboard");
        assert!(300. - scroll >= 12., "and its top is still on screen");
    }

    #[test]
    fn oversized_nodes_align_to_their_top() {
        assert_eq!(scroll_to_reveal(0., 300., 900., 400., 16.), 284.);
    }
}
