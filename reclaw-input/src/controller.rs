use crate::action::Button;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ControllerKind {
    Xbox,
    PlayStation,
    Nintendo,
    SteamDeck,
    Generic,
}

impl ControllerKind {
    /// Vendor id wins; the name is the fallback. Note that Steam Input presents a *virtual Xbox 360
    /// pad* to apps, so under Steam the glyphs will read Xbox whatever hardware is in hand.
    pub fn detect(name: &str, vendor_id: Option<u16>) -> Self {
        match vendor_id {
            Some(0x045e) => return Self::Xbox,
            Some(0x054c) => return Self::PlayStation,
            Some(0x057e) => return Self::Nintendo,
            Some(0x28de) => return Self::SteamDeck,
            _ => {}
        }
        let name = name.to_lowercase();
        let has = |needles: &[&str]| needles.iter().any(|n| name.contains(n));
        if has(&["steam deck", "steam controller"]) {
            Self::SteamDeck
        } else if has(&["xbox", "x-box", "xinput"]) {
            Self::Xbox
        } else if has(&["dualsense", "dualshock", "playstation", "ps4", "ps5", "wireless controller"]) {
            Self::PlayStation
        } else if has(&["nintendo", "switch", "joy-con", "pro controller"]) {
            Self::Nintendo
        } else {
            Self::Generic
        }
    }

    pub fn glyph(self, button: Button) -> GlyphFace {
        use Button::*;
        use GlyphFace::Label as L;
        match self {
            Self::PlayStation => match button {
                South => GlyphFace::Cross,
                East => GlyphFace::Circle,
                West => GlyphFace::Square,
                North => GlyphFace::Triangle,
                LeftBumper => L("L1"),
                RightBumper => L("R1"),
                LeftTrigger => L("L2"),
                RightTrigger => L("R2"),
                Select => L("Create"),
                Start => L("Options"),
                Guide => L("PS"),
                LeftStick => L("L3"),
                RightStick => L("R3"),
                DPadUp | DPadDown | DPadLeft | DPadRight => L("D-pad"),
            },
            Self::Nintendo => match button {
                South => L("B"),
                East => L("A"),
                West => L("Y"),
                North => L("X"),
                LeftBumper => L("L"),
                RightBumper => L("R"),
                LeftTrigger => L("ZL"),
                RightTrigger => L("ZR"),
                Select => L("-"),
                Start => L("+"),
                Guide => L("Home"),
                LeftStick => L("LS"),
                RightStick => L("RS"),
                DPadUp | DPadDown | DPadLeft | DPadRight => L("D-pad"),
            },
            Self::Xbox | Self::SteamDeck | Self::Generic => {
                let deck = self != Self::Xbox;
                match button {
                    South => L("A"),
                    East => L("B"),
                    West => L("X"),
                    North => L("Y"),
                    LeftBumper => L(if deck { "L1" } else { "LB" }),
                    RightBumper => L(if deck { "R1" } else { "RB" }),
                    LeftTrigger => L(if deck { "L2" } else { "LT" }),
                    RightTrigger => L(if deck { "R2" } else { "RT" }),
                    Select => L("View"),
                    Start => L("Menu"),
                    Guide => L(if self == Self::SteamDeck { "Steam" } else { "Guide" }),
                    LeftStick => L("L3"),
                    RightStick => L("R3"),
                    DPadUp | DPadDown | DPadLeft | DPadRight => L("D-pad"),
                }
            }
        }
    }
}

/// What to draw for a button. PlayStation face buttons are shapes, not letters: the UI renders
/// them with icons so no font has to carry the symbols.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlyphFace {
    Label(&'static str),
    Cross,
    Circle,
    Square,
    Triangle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PowerState {
    #[default]
    Unknown,
    Wired,
    Charging(u8),
    Discharging(u8),
    Full,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ControllerInfo {
    pub id: u32,
    pub name: String,
    pub kind: ControllerKind,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub power: PowerState,
}

impl ControllerInfo {
    pub fn new(id: u32, name: impl Into<String>, vendor_id: Option<u16>, product_id: Option<u16>) -> Self {
        let name = name.into();
        let kind = ControllerKind::detect(&name, vendor_id);
        Self { id, name, kind, vendor_id, product_id, power: PowerState::Unknown }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_beats_name() {
        assert_eq!(ControllerKind::detect("Wireless Controller", Some(0x054c)), ControllerKind::PlayStation);
        assert_eq!(ControllerKind::detect("Totally Generic Pad", Some(0x045e)), ControllerKind::Xbox);
    }

    #[test]
    fn name_fallback() {
        assert_eq!(ControllerKind::detect("Nintendo Switch Pro Controller", None), ControllerKind::Nintendo);
        assert_eq!(ControllerKind::detect("Microsoft X-Box 360 pad", None), ControllerKind::Xbox);
        assert_eq!(ControllerKind::detect("8BitDo Lite", None), ControllerKind::Generic);
    }

    #[test]
    fn nintendo_labels_follow_the_cap_not_the_position() {
        assert_eq!(ControllerKind::Nintendo.glyph(Button::East), GlyphFace::Label("A"));
        assert_eq!(ControllerKind::Xbox.glyph(Button::East), GlyphFace::Label("B"));
        assert_eq!(ControllerKind::PlayStation.glyph(Button::South), GlyphFace::Cross);
    }
}
