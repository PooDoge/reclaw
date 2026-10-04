//! Focus ids. Tiles encode (shelf, game); everything else is a small fixed id or a base plus an
//! index, so a screen can be recognised from an id alone.
use reclaw_input::FocusId;

pub const BANNER_RESUME: FocusId = FocusId(1);
pub const BANNER_STOP: FocusId = FocusId(2);
pub const GAME_PRIMARY: FocusId = FocusId(10);
pub const GAME_STOP: FocusId = FocusId(11);
pub const GAME_FOLDER: FocusId = FocusId(12);
pub const GAME_MANAGE: FocusId = FocusId(13);
pub const QA_RESUME: FocusId = FocusId(20);
pub const QA_STOP: FocusId = FocusId(21);
pub const QA_DOWNLOADS: FocusId = FocusId(22);
pub const INSTALL_LOCATION: FocusId = FocusId(30);
pub const INSTALL_FILE: FocusId = FocusId(31);
pub const INSTALL_SHORTCUT: FocusId = FocusId(32);
pub const INSTALL_PRERELEASE: FocusId = FocusId(33);
pub const INSTALL_CANCEL: FocusId = FocusId(34);
pub const INSTALL_SUBMIT: FocusId = FocusId(35);
pub const CONFIRM_CANCEL: FocusId = FocusId(40);
pub const CONFIRM_OK: FocusId = FocusId(41);
pub const NOTICE_CLOSE: FocusId = FocusId(42);
pub const NOTICE_DISMISS: FocusId = FocusId(43);
/// The header's Back button on a page (reachable by pointer and touch; the pad uses B).
pub const PAGE_BACK: FocusId = FocusId(50);

const TILE: u32 = 0x1000_0000;
const MENU: u32 = 0x2000_0000;
const DOWNLOAD: u32 = 0x4000_0000;
const SETTINGS_NAV: u32 = 0x5000_0000;
const SETTINGS_ROW: u32 = 0x6000_0000;
const QA_RECENT: u32 = 0x7000_0000;
const KIND_MASK: u32 = 0xF000_0000;

pub fn tile(shelf: usize, game: u32) -> FocusId {
    FocusId(TILE | ((shelf as u32) << 20) | game)
}

pub fn menu(index: usize) -> FocusId {
    FocusId(MENU | index as u32)
}

/// The button of one Downloads row, by activity id (ids past 2^28 would collide with the tag bits).
pub fn download_cancel(activity: u64) -> FocusId {
    FocusId(DOWNLOAD | (activity as u32 & 0x0FFF_FFFF))
}

pub fn settings_nav(section: usize) -> FocusId {
    FocusId(SETTINGS_NAV | section as u32)
}

pub fn settings_row(section: usize, row: usize) -> FocusId {
    FocusId(SETTINGS_ROW | ((section as u32) << 12) | row as u32)
}

/// Game id out of a tile id, if it is one.
pub fn tile_game(id: FocusId) -> Option<u32> {
    (id.0 & KIND_MASK == TILE).then_some(id.0 & 0x000F_FFFF)
}

pub fn tile_shelf(id: FocusId) -> Option<usize> {
    (id.0 & KIND_MASK == TILE).then_some(((id.0 >> 20) & 0xFF) as usize)
}

/// A row of the Quick access panel's recent pages.
pub fn qa_recent(index: usize) -> FocusId {
    FocusId(QA_RECENT | index as u32)
}

pub fn qa_recent_index(id: FocusId) -> Option<usize> {
    (id.0 & KIND_MASK == QA_RECENT).then_some((id.0 & 0xFFFF) as usize)
}

pub fn menu_index(id: FocusId) -> Option<usize> {
    (id.0 & KIND_MASK == MENU).then_some((id.0 & 0xFFFF) as usize)
}

pub fn cancel_activity(id: FocusId) -> Option<u64> {
    (id.0 & KIND_MASK == DOWNLOAD).then_some(u64::from(id.0 & 0x0FFF_FFFF))
}

pub fn nav_section(id: FocusId) -> Option<usize> {
    (id.0 & KIND_MASK == SETTINGS_NAV).then_some((id.0 & 0xFFFF) as usize)
}

/// (section, row) of a settings row id.
pub fn row_of(id: FocusId) -> Option<(usize, usize)> {
    (id.0 & KIND_MASK == SETTINGS_ROW).then_some((((id.0 >> 12) & 0xFFFF) as usize, (id.0 & 0xFFF) as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        assert_eq!(tile_game(tile(1, 42)), Some(42));
        assert_eq!(tile_shelf(tile(1, 42)), Some(1));
        assert_eq!(menu_index(menu(5)), Some(5));
        assert_eq!(cancel_activity(download_cancel(9)), Some(9));
        assert_eq!(nav_section(settings_nav(3)), Some(3));
        assert_eq!(row_of(settings_row(2, 7)), Some((2, 7)));
    }

    #[test]
    fn kinds_do_not_alias() {
        assert_eq!(tile_game(menu(1)), None);
        assert_eq!(row_of(settings_nav(1)), None);
        assert_eq!(nav_section(settings_row(0, 1)), None);
        assert_eq!(menu_index(INSTALL_FILE), None);
    }
}
