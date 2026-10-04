//! Where each settings row sits, in the scrolling content's own coordinates. The focus layout and
//! the renderer both read this, so they cannot drift apart.
use super::schema::Section;
use crate::{metrics::*, surface::row_height};

pub const ROW_GAP: f32 = 8.;
/// A group heading, plus its note when there is one.
pub const HEADING_H: f32 = 48.;
pub const NOTE_H: f32 = 32.;
pub const GROUP_GAP: f32 = 24.;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RowSlot {
    pub row: usize,
    pub y: f32,
    pub h: f32,
}

/// Every row of a section, in order, and the total content height.
pub fn section_slots(section: &Section, density: Density) -> (Vec<RowSlot>, f32) {
    let mut slots = Vec::new();
    let mut y = 0.;
    let mut index = 0;
    for (g, group) in section.groups.iter().enumerate() {
        if g > 0 {
            y += GROUP_GAP;
        }
        if group.heading.is_some() {
            y += HEADING_H;
        }
        if group.note.is_some() {
            y += NOTE_H;
        }
        for row in &group.rows {
            let h = row_height(row.is_text(), density);
            slots.push(RowSlot { row: index, y, h });
            y += h + ROW_GAP;
            index += 1;
        }
    }
    (slots, (y - ROW_GAP).max(0.))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::settings::{builders::global_settings, schema::RowKind};

    #[test]
    fn rows_are_stacked_without_overlap() {
        let schema = global_settings(&[]);
        for section in &schema.sections {
            let (slots, total) = section_slots(section, Density::Controller);
            assert_eq!(slots.len(), section.rows().count());
            for pair in slots.windows(2) {
                assert!(pair[1].y >= pair[0].y + pair[0].h, "rows overlap in {}", section.id);
            }
            let last = slots.last().unwrap();
            assert_eq!(total, last.y + last.h);
        }
    }

    #[test]
    fn text_rows_are_taller() {
        let schema = global_settings(&[]);
        let library = schema.sections.iter().find(|s| s.id == "library").unwrap();
        let (slots, _) = section_slots(library, Density::Controller);
        let text = library.rows().position(|r| matches!(r.kind, RowKind::Text { .. })).unwrap();
        assert!(slots[text].h > slots[0].h);
    }

    #[test]
    fn group_headings_push_their_rows_down() {
        let schema = global_settings(&[]);
        let library = schema.sections.iter().find(|s| s.id == "library").unwrap();
        let (slots, _) = section_slots(library, Density::Controller);
        // Second group has a heading: its row starts after the first row, the gap and the heading.
        assert_eq!(slots[1].y, slots[0].y + slots[0].h + ROW_GAP + GROUP_GAP + HEADING_H);
    }
}
