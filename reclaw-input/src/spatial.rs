use crate::action::Direction;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct FocusId(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn cx(&self) -> f32 {
        self.x + self.w / 2.
    }

    fn cy(&self) -> f32 {
        self.y + self.h / 2.
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FocusNode {
    pub id: FocusId,
    pub rect: Rect,
}

/// Directional focus move over declared rectangles.
///
/// A candidate must lie beyond the current center in the pressed direction. Candidates are scored
/// by distance along that axis plus a heavy penalty for sideways offset (and a heavier one for
/// not overlapping at all), so pressing Down from a tile lands on the tile *under* it, not on
/// the nearest tile in the next row. Returns `None` at an edge; the caller decides whether to
/// leave the scope.
pub fn next_focus(nodes: &[FocusNode], current: FocusId, dir: Direction) -> Option<FocusId> {
    let from = nodes.iter().find(|n| n.id == current)?.rect;
    let mut best: Option<(f32, FocusId)> = None;

    for node in nodes.iter().filter(|n| n.id != current) {
        let r = node.rect;
        let (along, cross_center, cross_gap) = match dir {
            Direction::Right => (r.cx() - from.cx(), (r.cy() - from.cy()).abs(), gap(from.y, from.h, r.y, r.h)),
            Direction::Left => (from.cx() - r.cx(), (r.cy() - from.cy()).abs(), gap(from.y, from.h, r.y, r.h)),
            Direction::Down => (r.cy() - from.cy(), (r.cx() - from.cx()).abs(), gap(from.x, from.w, r.x, r.w)),
            Direction::Up => (from.cy() - r.cy(), (r.cx() - from.cx()).abs(), gap(from.x, from.w, r.x, r.w)),
        };
        if along <= 0.5 {
            continue;
        }
        let score = along + 2. * cross_center + 4. * cross_gap;
        let better = match best {
            None => true,
            Some((s, id)) => score < s || (score == s && node.id < id),
        };
        if better {
            best = Some((score, node.id));
        }
    }
    best.map(|(_, id)| id)
}

/// Distance between two 1-D intervals; 0 when they overlap.
fn gap(a_start: f32, a_len: f32, b_start: f32, b_len: f32) -> f32 {
    let a_end = a_start + a_len;
    let b_end = b_start + b_len;
    (b_start - a_end).max(a_start - b_end).max(0.)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u32, x: f32, y: f32, w: f32, h: f32) -> FocusNode {
        FocusNode { id: FocusId(id), rect: Rect::new(x, y, w, h) }
    }

    /// Two shelves of tiles; the second shelf is shifted by half a tile.
    fn shelves() -> Vec<FocusNode> {
        vec![
            node(1, 0., 0., 100., 100.),
            node(2, 110., 0., 100., 100.),
            node(3, 220., 0., 100., 100.),
            node(11, 55., 120., 100., 100.),
            node(12, 165., 120., 100., 100.),
        ]
    }

    #[test]
    fn moves_along_a_row() {
        let n = shelves();
        assert_eq!(next_focus(&n, FocusId(1), Direction::Right), Some(FocusId(2)));
        assert_eq!(next_focus(&n, FocusId(3), Direction::Left), Some(FocusId(2)));
    }

    #[test]
    fn stops_at_edges() {
        let n = shelves();
        assert_eq!(next_focus(&n, FocusId(1), Direction::Left), None);
        assert_eq!(next_focus(&n, FocusId(3), Direction::Right), None);
        assert_eq!(next_focus(&n, FocusId(1), Direction::Up), None);
    }

    #[test]
    fn down_lands_on_the_tile_underneath() {
        let n = shelves();
        // Tile 2 (x 110..210) sits between 11 (55..155) and 12 (165..265); 11's center is 55 away,
        // 12's is 55 away: equal, so the lower id wins deterministically.
        assert_eq!(next_focus(&n, FocusId(2), Direction::Down), Some(FocusId(11)));
        assert_eq!(next_focus(&n, FocusId(3), Direction::Down), Some(FocusId(12)));
        // 12 sits exactly between 2 and 3: a tie goes to the lower id, deterministically.
        assert_eq!(next_focus(&n, FocusId(12), Direction::Up), Some(FocusId(2)));
    }

    #[test]
    fn unknown_current_is_none() {
        assert_eq!(next_focus(&shelves(), FocusId(99), Direction::Right), None);
    }

    #[test]
    fn prefers_aligned_over_nearer_but_offset() {
        let n = vec![
            node(1, 0., 0., 100., 100.),
            node(2, 120., 0., 100., 100.),  // aligned, 120 away
            node(3, 60., 400., 100., 100.), // not to the right at all
            node(4, 110., 90., 100., 100.), // overlaps vertically by 10 only
        ];
        assert_eq!(next_focus(&n, FocusId(1), Direction::Right), Some(FocusId(2)));
    }
}
