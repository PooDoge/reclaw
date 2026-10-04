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
    WiiU,
    GameBoy,
    Gba,
    Ds,
    N3ds,
    Ps1,
    Ps2,
    Psp,
    Xbox,
    Xbox360,
    Genesis,
    Saturn,
    Dreamcast,
    /// Coin-operated cabinets and the boards inside them.
    Arcade,
    /// A game that began on a personal computer (Windows or DOS), including fan recreations.
    Pc,
    /// A phone or tablet game, including the feature-phone platforms (BREW, J2ME).
    Mobile,
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
    Arcade,
    Computer,
    Phone,
    Other,
}

/// Every system, in sort order.
pub const ALL: [Platform; 22] = [
    Platform::Nes,
    Platform::Snes,
    Platform::N64,
    Platform::GameCube,
    Platform::Wii,
    Platform::WiiU,
    Platform::GameBoy,
    Platform::Gba,
    Platform::Ds,
    Platform::N3ds,
    Platform::Ps1,
    Platform::Ps2,
    Platform::Psp,
    Platform::Xbox,
    Platform::Xbox360,
    Platform::Genesis,
    Platform::Saturn,
    Platform::Dreamcast,
    Platform::Arcade,
    Platform::Pc,
    Platform::Mobile,
    Platform::Other,
];

impl Platform {
    pub const ALL: [Platform; 22] = ALL;

    /// The full name, for headings and menus.
    pub fn label(self) -> &'static str {
        match self {
            Self::Nes => "NES",
            Self::Snes => "Super Nintendo",
            Self::N64 => "Nintendo 64",
            Self::GameCube => "GameCube",
            Self::Wii => "Wii",
            Self::WiiU => "Wii U",
            Self::GameBoy => "Game Boy",
            Self::Gba => "Game Boy Advance",
            Self::Ds => "Nintendo DS",
            Self::N3ds => "Nintendo 3DS",
            Self::Ps1 => "PlayStation",
            Self::Ps2 => "PlayStation 2",
            Self::Psp => "PSP",
            Self::Xbox => "Xbox",
            Self::Xbox360 => "Xbox 360",
            Self::Genesis => "Genesis",
            Self::Saturn => "Saturn",
            Self::Dreamcast => "Dreamcast",
            Self::Arcade => "Arcade",
            Self::Pc => "PC",
            Self::Mobile => "Mobile",
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
            Self::WiiU => "WiiU",
            Self::GameBoy => "GB",
            Self::Gba => "GBA",
            Self::Ds => "NDS",
            Self::N3ds => "3DS",
            Self::Ps1 => "PS1",
            Self::Ps2 => "PS2",
            Self::Psp => "PSP",
            Self::Xbox => "XBOX",
            Self::Xbox360 => "X360",
            Self::Genesis => "GEN",
            Self::Saturn => "SAT",
            Self::Dreamcast => "DC",
            Self::Arcade => "ARC",
            Self::Pc => "PC",
            Self::Mobile => "MOB",
            Self::Other => "Other",
        }
    }

    pub fn maker(self) -> Maker {
        match self {
            Self::Nes
            | Self::Snes
            | Self::N64
            | Self::GameCube
            | Self::Wii
            | Self::WiiU
            | Self::GameBoy
            | Self::Gba
            | Self::Ds
            | Self::N3ds => Maker::Nintendo,
            Self::Ps1 | Self::Ps2 | Self::Psp => Maker::Sony,
            Self::Xbox | Self::Xbox360 => Maker::Microsoft,
            Self::Genesis | Self::Saturn | Self::Dreamcast => Maker::Sega,
            Self::Arcade | Self::Pc | Self::Mobile | Self::Other => Maker::Other,
        }
    }

    pub fn kind(self) -> SystemKind {
        match self {
            Self::GameBoy | Self::Gba | Self::Ds | Self::N3ds | Self::Psp => SystemKind::Handheld,
            Self::Arcade => SystemKind::Arcade,
            Self::Pc => SystemKind::Computer,
            Self::Mobile => SystemKind::Phone,
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

    /// The system a tag names, ignoring case and spacing: "n64", "GameCube", "game boy advance". A tag that only
    /// names a brand ("playstation", "xbox") gives the system the brand began with; use [`Platform::from_tags`] for
    /// an app's whole tag list, which weighs the tags against each other.
    pub fn from_tag(tag: &str) -> Option<Self> {
        classify(tag).map(|(platform, _)| platform)
    }

    /// The system an app came from, given all its tags. Catalogs tag generously (a PlayStation 2 game carries
    /// "playstation", "ps2" and "playstation 2"; an Xbox 360 game carries "xbox", "x360" and "xbox 360"), so the
    /// first tag is not the answer. A tag naming one system beats a brand tag, and either beats a tag for the
    /// machine it now runs on ("pc"); ties go to the tag listed first.
    pub fn from_tags<I, S>(tags: I) -> Option<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        tags.into_iter().filter_map(|t| classify(t.as_ref())).min_by_key(|(_, strength)| *strength).map(|(platform, _)| platform)
    }
}

/// How well a tag pins down a system. Lower is stronger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Strength {
    /// Names one system: "n64", "ps2", "x360".
    System,
    /// Names a maker's line and could mean several: "playstation", "xbox".
    Brand,
    /// Names where a game can be played today rather than where it came from: "pc", "mobile".
    Host,
}

fn classify(tag: &str) -> Option<(Platform, Strength)> {
    use Platform as P;
    use Strength::{Brand, Host, System};
    let key: String = tag.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    Some(match key.as_str() {
        "nes" | "famicom" | "nintendoentertainmentsystem" => (P::Nes, System),
        "snes" | "supernintendo" | "supernes" | "supernintendoentertainmentsystem" | "superfamicom" => (P::Snes, System),
        "n64" | "nintendo64" => (P::N64, System),
        "gamecube" | "gcn" | "gc" | "ngc" => (P::GameCube, System),
        "wii" => (P::Wii, System),
        "wiiu" | "nintendowiiu" => (P::WiiU, System),
        "gb" | "gbc" | "gameboy" | "gameboycolor" => (P::GameBoy, System),
        "gba" | "gameboyadvance" => (P::Gba, System),
        "ds" | "nds" | "nintendods" => (P::Ds, System),
        "3ds" | "n3ds" | "nintendo3ds" => (P::N3ds, System),
        "ps1" | "psx" | "psone" | "playstation1" => (P::Ps1, System),
        "ps2" | "playstation2" => (P::Ps2, System),
        "psp" | "playstationportable" => (P::Psp, System),
        "playstation" => (P::Ps1, Brand),
        "originalxbox" | "xboxog" => (P::Xbox, System),
        "xbox" => (P::Xbox, Brand),
        "x360" | "xbox360" => (P::Xbox360, System),
        "genesis" | "megadrive" | "md" | "gen" | "smd" | "segagenesis" | "segamegadrive" => (P::Genesis, System),
        "saturn" | "sat" | "segasaturn" => (P::Saturn, System),
        "dreamcast" | "dc" | "segadreamcast" => (P::Dreamcast, System),
        "arcade" | "arc" => (P::Arcade, System),
        "pc" | "windows" | "dos" | "msdos" => (P::Pc, Host),
        "mobile" | "mob" | "android" | "ios" | "brew" | "j2me" => (P::Mobile, Host),
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
