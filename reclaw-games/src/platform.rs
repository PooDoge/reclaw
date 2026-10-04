//! The systems a game was recompiled from: what they are called, who made them, how they sort, and
//! how a free-text tag ("n64", "GameCube") maps to one. Pure data, shared by the catalog, the library
//! and the game page.
use serde::{Deserialize, Serialize};

/// The console or handheld the original game ran on.
///
/// The variants are listed in the order the library sorts "by system": grouped by maker, and within
/// a maker by when the system came out. Adding a system means adding a variant here, to [`ALL`], and
/// to each match below; the compiler points at all of them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Nes,
    Snes,
    N64,
    GameCube,
    Wii,
    GameBoy,
    Gba,
    Ds,
    Ps1,
    Ps2,
    Psp,
    Xbox,
    Genesis,
    Saturn,
    Dreamcast,
    Other,
}

/// Who made the system. Only used to group and to order; no logos or brand colors.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Maker {
    Nintendo,
    Sony,
    Microsoft,
    Sega,
    Other,
}

impl Maker {
    pub fn label(self) -> &'static str {
        match self {
            Self::Nintendo => "Nintendo",
            Self::Sony => "Sony",
            Self::Microsoft => "Microsoft",
            Self::Sega => "Sega",
            Self::Other => "Other",
        }
    }
}

/// Whether the system plugged into a television or fit in a hand.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SystemKind {
    Console,
    Handheld,
    Other,
}

/// Every system, in sort order.
pub const ALL: [Platform; 16] = [
    Platform::Nes,
    Platform::Snes,
    Platform::N64,
    Platform::GameCube,
    Platform::Wii,
    Platform::GameBoy,
    Platform::Gba,
    Platform::Ds,
    Platform::Ps1,
    Platform::Ps2,
    Platform::Psp,
    Platform::Xbox,
    Platform::Genesis,
    Platform::Saturn,
    Platform::Dreamcast,
    Platform::Other,
];

impl Platform {
    pub const ALL: [Platform; 16] = ALL;

    /// The full name, for headings and menus.
    pub fn label(self) -> &'static str {
        match self {
            Self::Nes => "NES",
            Self::Snes => "Super Nintendo",
            Self::N64 => "Nintendo 64",
            Self::GameCube => "GameCube",
            Self::Wii => "Wii",
            Self::GameBoy => "Game Boy",
            Self::Gba => "Game Boy Advance",
            Self::Ds => "Nintendo DS",
            Self::Ps1 => "PlayStation",
            Self::Ps2 => "PlayStation 2",
            Self::Psp => "PSP",
            Self::Xbox => "Xbox",
            Self::Genesis => "Genesis",
            Self::Saturn => "Saturn",
            Self::Dreamcast => "Dreamcast",
            Self::Other => "Other",
        }
    }

    /// The short mark people know the system by, for a badge on a card: at most four characters,
    /// except "Other", which is never put on a card (see [`Platform::is_known`]).
    pub fn short(self) -> &'static str {
        match self {
            Self::Nes => "NES",
            Self::Snes => "SNES",
            Self::N64 => "N64",
            Self::GameCube => "GCN",
            Self::Wii => "Wii",
            Self::GameBoy => "GB",
            Self::Gba => "GBA",
            Self::Ds => "NDS",
            Self::Ps1 => "PS1",
            Self::Ps2 => "PS2",
            Self::Psp => "PSP",
            Self::Xbox => "XBOX",
            Self::Genesis => "GEN",
            Self::Saturn => "SAT",
            Self::Dreamcast => "DC",
            Self::Other => "Other",
        }
    }

    pub fn maker(self) -> Maker {
        match self {
            Self::Nes | Self::Snes | Self::N64 | Self::GameCube | Self::Wii | Self::GameBoy | Self::Gba | Self::Ds => Maker::Nintendo,
            Self::Ps1 | Self::Ps2 | Self::Psp => Maker::Sony,
            Self::Xbox => Maker::Microsoft,
            Self::Genesis | Self::Saturn | Self::Dreamcast => Maker::Sega,
            Self::Other => Maker::Other,
        }
    }

    pub fn kind(self) -> SystemKind {
        match self {
            Self::GameBoy | Self::Gba | Self::Ds | Self::Psp => SystemKind::Handheld,
            Self::Other => SystemKind::Other,
            _ => SystemKind::Console,
        }
    }

    /// Whether this names a real system, as opposed to the catch-all. A card shows a badge only for these.
    pub fn is_known(self) -> bool {
        self != Self::Other
    }

    /// Where the system sits in the sort order.
    pub fn rank(self) -> usize {
        ALL.iter().position(|p| *p == self).unwrap_or(ALL.len())
    }

    /// The system a tag names, ignoring case and spacing: "n64", "GameCube", "game boy advance".
    pub fn from_tag(tag: &str) -> Option<Self> {
        let key: String = tag.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
        Some(match key.as_str() {
            "nes" | "famicom" => Self::Nes,
            "snes" | "supernintendo" | "supernes" => Self::Snes,
            "n64" | "nintendo64" => Self::N64,
            "gamecube" | "gcn" | "gc" => Self::GameCube,
            "wii" => Self::Wii,
            "gb" | "gbc" | "gameboy" | "gameboycolor" => Self::GameBoy,
            "gba" | "gameboyadvance" => Self::Gba,
            "ds" | "nds" | "nintendods" => Self::Ds,
            "ps1" | "psx" | "psone" | "playstation" | "playstation1" => Self::Ps1,
            "ps2" | "playstation2" => Self::Ps2,
            "psp" | "playstationportable" => Self::Psp,
            "xbox" | "originalxbox" => Self::Xbox,
            "genesis" | "megadrive" | "md" | "gen" => Self::Genesis,
            "saturn" | "sat" => Self::Saturn,
            "dreamcast" | "dc" => Self::Dreamcast,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests;
