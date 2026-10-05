//! Settings as data: sections of groups of rows. Pages render a schema; the reducer edits values
//! by key. Nothing here knows about Freya.
use reclaw_games::settings::SettingKey;
use reclaw_net::Provider;

/// Which text box a gamepad press should start typing into.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TextField {
    InstallLocation,
    LaunchOptions,
    SdlOverride,
    DefaultLocation,
    /// A token being typed. Its text is never stored as a setting or passed around as a plain string: see `Effect::SaveToken`.
    GithubToken,
    GitlabToken,
}

impl TextField {
    /// The box a service's token is typed into.
    pub fn for_provider(provider: Provider) -> Self {
        match provider {
            Provider::GitHub => Self::GithubToken,
            Provider::GitLab => Self::GitlabToken,
        }
    }

    /// The service this box holds a token for, if it is a token box.
    pub fn token_provider(self) -> Option<Provider> {
        match self {
            Self::GithubToken => Some(Provider::GitHub),
            Self::GitlabToken => Some(Provider::GitLab),
            _ => None,
        }
    }

    /// Text typed here is shown as dots and never saved with the settings.
    pub fn is_secret(self) -> bool {
        self.token_provider().is_some()
    }

    /// The settings key the text is stored under, or `None` for a text that belongs to a form, not
    /// to the settings (the install location is typed fresh for each install).
    pub fn key(self) -> Option<&'static str> {
        match self {
            Self::InstallLocation | Self::GithubToken | Self::GitlabToken => None,
            Self::LaunchOptions => Some("launch_options"),
            Self::SdlOverride => Some("sdl_override"),
            Self::DefaultLocation => Some(super::location::KEY_DEFAULT_LOCATION),
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

/// The rows of a service's access-token group, top to bottom.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CredentialPart {
    /// Read-only: what is known about the token.
    Status,
    /// The box the token is pasted into.
    Token,
    /// Keep what was pasted.
    Save,
    Check,
    /// Open the service's page for making a token.
    Create,
    Remove,
}

impl CredentialPart {
    /// The key of this row (unique within the Network section).
    pub fn key(self, provider: Provider) -> &'static str {
        match (provider, self) {
            (Provider::GitHub, Self::Status) => "github_status",
            (Provider::GitHub, Self::Token) => "github_token",
            (Provider::GitHub, Self::Save) => "github_save",
            (Provider::GitHub, Self::Check) => "github_check",
            (Provider::GitHub, Self::Create) => "github_create",
            (Provider::GitHub, Self::Remove) => "github_remove",
            (Provider::GitLab, Self::Status) => "gitlab_status",
            (Provider::GitLab, Self::Token) => "gitlab_token",
            (Provider::GitLab, Self::Save) => "gitlab_save",
            (Provider::GitLab, Self::Check) => "gitlab_check",
            (Provider::GitLab, Self::Create) => "gitlab_create",
            (Provider::GitLab, Self::Remove) => "gitlab_remove",
        }
    }
}

/// What a global action row does (rows about Reclaw itself rather than one app).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlobalAction {
    OpenLogFolder,
    SaveDiagnostics,
    /// Fetch the newest commits of the checkout this copy was built from, and rebuild.
    UpdateSources,
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
    /// One row of a service's access-token group. What each shows comes from the store's credentials status, so the schema stays
    /// the same for everyone and the text is worked out when it is drawn.
    Credential {
        provider: Provider,
        part: CredentialPart,
    },
    /// An action on Reclaw itself.
    Global {
        action: GlobalAction,
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

    /// Whether the row has a text box under its label (and so is taller).
    pub fn is_text(&self) -> bool {
        matches!(self.kind, RowKind::Text { .. } | RowKind::Credential { part: CredentialPart::Token, .. })
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
