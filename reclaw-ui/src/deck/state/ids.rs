//! Focus ids. The low 32 bits hold a kind tag and a small index; a tile also carries its game's id,
//! whole, in the high 32 bits (game ids are 32-bit hashes, so they cannot share the low word).
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
pub const INSTALL_PRERELEASE: FocusId = FocusId(33);
pub const INSTALL_CANCEL: FocusId = FocusId(34);
pub const INSTALL_SUBMIT: FocusId = FocusId(35);
pub const CONFIRM_CANCEL: FocusId = FocusId(40);
pub const CONFIRM_OK: FocusId = FocusId(41);
pub const NOTICE_CLOSE: FocusId = FocusId(42);
pub const NOTICE_DISMISS: FocusId = FocusId(43);
/// The header's Back button on a page (reachable by pointer and touch; the pad uses B).
pub const PAGE_BACK: FocusId = FocusId(50);

const TILE: u64 = 0x1000_0000;
const MENU: u64 = 0x2000_0000;
const DOWNLOAD: u64 = 0x4000_0000;
const SETTINGS_NAV: u64 = 0x5000_0000;
const SETTINGS_ROW: u64 = 0x6000_0000;
const QA_RECENT: u64 = 0x7000_0000;
/// The kind tag. The high word is left out: there a tile keeps its game id.
const KIND_MASK: u64 = 0xF000_0000;

pub fn tile(shelf: usize, game: u32) -> FocusId {
    FocusId((u64::from(game) << 32) | TILE | (((shelf as u64) & 0xFF) << 20))
}

pub fn menu(index: usize) -> FocusId {
    FocusId(MENU | index as u64)
}

/// The button of one Downloads row, by activity id (ids past 2^28 would collide with the tag bits).
pub fn download_cancel(activity: u64) -> FocusId {
    FocusId(DOWNLOAD | (activity & 0x0FFF_FFFF))
}

pub fn settings_nav(section: usize) -> FocusId {
    FocusId(SETTINGS_NAV | section as u64)
}

pub fn settings_row(section: usize, row: usize) -> FocusId {
    FocusId(SETTINGS_ROW | ((section as u64) << 12) | row as u64)
}

/// Game id out of a tile id, if it is one.
pub fn tile_game(id: FocusId) -> Option<u32> {
    (id.0 & KIND_MASK == TILE).then_some((id.0 >> 32) as u32)
}

pub fn tile_shelf(id: FocusId) -> Option<usize> {
    (id.0 & KIND_MASK == TILE).then_some(((id.0 >> 20) & 0xFF) as usize)
}

/// A row of the Quick access panel's recent pages.
pub fn qa_recent(index: usize) -> FocusId {
    FocusId(QA_RECENT | index as u64)
}

pub fn qa_recent_index(id: FocusId) -> Option<usize> {
    (id.0 & KIND_MASK == QA_RECENT).then_some((id.0 & 0xFFFF) as usize)
}

pub fn menu_index(id: FocusId) -> Option<usize> {
    (id.0 & KIND_MASK == MENU).then_some((id.0 & 0xFFFF) as usize)
}

pub fn cancel_activity(id: FocusId) -> Option<u64> {
    (id.0 & KIND_MASK == DOWNLOAD).then_some(id.0 & 0x0FFF_FFFF)
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
        assert_eq!(menu_index(INSTALL_PRERELEASE), None);
    }

    #[test]
    fn a_tile_keeps_a_full_32_bit_game_id_and_its_shelf() {
        // Catalog ids are FNV hashes: any bit can be set, including the ones the kind tag and shelf use.
        for game in [0xFFFF_FFFF, 0x9E37_79B9, 0x0010_0000, 0x4000_0001] {
            for shelf in [0, 1, 7] {
                let id = tile(shelf, game);
                assert_eq!((tile_game(id), tile_shelf(id)), (Some(game), Some(shelf)), "{game:#x} on shelf {shelf}");
                assert_eq!((menu_index(id), nav_section(id), cancel_activity(id)), (None, None, None));
            }
        }
        assert_ne!(tile(0, 0x0010_0000), tile(1, 0x0010_0000), "one game on two shelves is two targets");
    }
}
