//! Lucide icons by role. Names are the ones listed in reclaw.freya.json; the compiler checks
//! that each exists in the pinned freya-icons.
use freya::prelude::*;
use freya_icons::lucide;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IconName {
    Play,
    Download,
    Check,
    Alert,
    X,
    Search,
    Library,
    Catalog,
    Mods,
    Queue,
    Settings,
    Folder,
    Chevron,
    Refresh,
    File,
    Stop,
    Circle,
    Square,
    Triangle,
    Desktop,
}

impl IconName {
    fn source(self) -> (&'static str, bytes::Bytes) {
        match self {
            Self::Play => ("play", lucide::play()),
            Self::Download => ("download", lucide::download()),
            Self::Check => ("check", lucide::check()),
            Self::Alert => ("triangle-alert", lucide::triangle_alert()),
            Self::X => ("x", lucide::x()),
            Self::Search => ("search", lucide::search()),
            Self::Library => ("library-big", lucide::library_big()),
            Self::Catalog => ("store", lucide::store()),
            Self::Mods => ("puzzle", lucide::puzzle()),
            Self::Queue => ("arrow-down-to-line", lucide::arrow_down_to_line()),
            Self::Settings => ("settings", lucide::settings()),
            Self::Folder => ("folder-open", lucide::folder_open()),
            Self::Chevron => ("chevron-right", lucide::chevron_right()),
            Self::Refresh => ("refresh-cw", lucide::refresh_cw()),
            Self::File => ("file", lucide::file()),
            Self::Stop => ("square", lucide::square()),
            Self::Circle => ("circle", lucide::circle()),
            Self::Square => ("square", lucide::square()),
            Self::Triangle => ("triangle", lucide::triangle()),
            Self::Desktop => ("monitor", lucide::monitor()),
        }
    }
}

pub fn icon(name: IconName, size: f32, color: Color) -> SvgViewer {
    SvgViewer::new(name.source())
        .color(color)
        .width(Size::px(size))
        .height(Size::px(size))
        .show_loader(false)
}
