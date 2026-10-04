//! Settings as data: sections of groups of rows. Pages render a schema; the reducer edits values
//! by key. Nothing here knows about Freya.
use reclaw_games::settings::SettingKey;

/// Which text box a gamepad press should start typing into.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TextField {
    InstallLocation,
    LaunchOptions,
    SdlOverride,
    DefaultLocation,
}

impl TextField {
    /// The settings key the text is stored under, or `None` for a text that belongs to a form, not
    /// to the settings (the install location is typed fresh for each install).
    pub fn key(self) -> Option<&'static str> {
        match self {
            Self::InstallLocation => None,
            Self::LaunchOptions => Some("launch_options"),
            Self::SdlOverride => Some("sdl_override"),
            Self::DefaultLocation => Some("default_location"),
        }
    }
}

/// What an action row does when pressed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RowAction {
    OpenFolder,
    Verify,
    CheckUpdate,
    /// Goes through a confirmation first.
    Uninstall,
}

#[derive(Clone, PartialEq, Debug)]
pub enum RowKind {
    Toggle {
        default: bool,
    },
    Choice {
        options: &'static [&'static str],
        default: usize,
    },
    Text {
        field: TextField,
        placeholder: &'static str,
    },
    Action {
        action: RowAction,
        danger: bool,
    },
    /// Read-only value.
    Info {
        value: String,
    },
    /// A launch setting from the games engine: its options, its value and where the value comes
    /// from depend on the display and the game, so they are worked out when shown (see `launch`).
    Launch {
        key: SettingKey,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub struct Row {
    pub key: &'static str,
    pub label: &'static str,
    pub description: Option<&'static str>,
    pub kind: RowKind,
}

impl Row {
    pub fn new(key: &'static str, label: &'static str, kind: RowKind) -> Self {
        Self { key, label, description: None, kind }
    }

    pub fn described(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn is_text(&self) -> bool {
        matches!(self.kind, RowKind::Text { .. })
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Group {
    pub heading: Option<&'static str>,
    pub note: Option<&'static str>,
    pub rows: Vec<Row>,
}

impl Group {
    pub fn new(rows: Vec<Row>) -> Self {
        Self { heading: None, note: None, rows }
    }

    pub fn headed(mut self, heading: &'static str) -> Self {
        self.heading = Some(heading);
        self
    }

    pub fn noted(mut self, note: &'static str) -> Self {
        self.note = Some(note);
        self
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Section {
    pub id: &'static str,
    pub title: &'static str,
    pub groups: Vec<Group>,
}

impl Section {
    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.groups.iter().flat_map(|g| g.rows.iter())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Schema {
    pub title: String,
    pub sections: Vec<Section>,
}

impl Schema {
    pub fn row(&self, section: usize, row: usize) -> Option<&Row> {
        self.sections.get(section)?.rows().nth(row)
    }
}
